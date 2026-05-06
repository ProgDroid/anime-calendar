//! HTTP routes for co-editor sharing: invitation lifecycle (owner-side and
//! invitee-side) plus member management.
//!
//! Controllers are thin shims — all business logic lives in
//! [`InvitationService`] and [`SharingAuthz`]. The controller's
//! responsibilities are:
//!   1. Load the parent `Calendar` (or short-circuit `404`).
//!   2. Run the appropriate `SharingAuthz::assert_can` gate.
//!   3. Translate `Result<T, Error>` into an `HttpResponse`.
//!
//! Anti-enumeration is delivered via `Error::InvalidRequest` for token
//! endpoints — see `services::invitation_service` module-level docs.

use actix_web::{HttpResponse, ResponseError, delete, get, post, web};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::config::server::SharingConfig;
use crate::entity::calendar_invitation::CalendarInvitation;
use crate::error::Error;
use crate::mappers::calendar_editor::CalendarEditorMapper;
use crate::mappers::calendar_invitation::CalendarInvitationMapper;
use crate::mappers::user::UserMapper;
use crate::middleware::auth::Claims;
use crate::services::invitation_service::{InvitationPreview, InvitationService};
use crate::services::sharing_authz::{Action, SharingAuthz};

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateInvitationRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MemberSummary {
    pub user_id: i32,
    pub display: String,
    pub email: String,
    pub joined_at: chrono::NaiveDateTime,
    pub suspended_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PendingInviteSummary {
    pub id: i64,
    pub invitee_email: String,
    pub sent_at: chrono::NaiveDateTime,
    pub expires_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MembersResponse {
    pub editors: Vec<MemberSummary>,
    pub pending: Vec<PendingInviteSummary>,
    pub editor_cap: u32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EmptyOk {
    pub ok: bool,
}

const fn ok_json() -> EmptyOk {
    EmptyOk { ok: true }
}

/// Resolve a calendar by id, treating "row missing" as `404 NotFound` and
/// every other failure as the underlying error. Used by every owner-side
/// route below.
async fn load_calendar_any_owner(
    pool: &sqlx::PgPool,
    id: i32,
) -> Result<crate::entity::calendar::Calendar, Error> {
    use crate::entity::calendar::Language;
    let row = sqlx::query!(
        "SELECT id, language as \"language: Language\", name, subscription_token, user_id,
                created_at, updated_at, event_style, frozen_subscribe_ics, meta_version
         FROM calendars
         WHERE id = $1 AND deleted_at IS NULL",
        id,
    )
    .fetch_optional(pool)
    .await?
    .ok_or(Error::NotFound)?;

    let item_ids: Vec<i32> = sqlx::query_scalar!(
        "SELECT item_id FROM calendar_items WHERE calendar_id = $1",
        id,
    )
    .fetch_all(pool)
    .await?;

    Ok(crate::entity::calendar::Calendar {
        id: row.id,
        item_ids,
        language: row.language,
        name: row.name,
        subscription_token: row.subscription_token,
        user_id: row.user_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
        event_style: row.event_style,
        frozen_subscribe_ics: row.frozen_subscribe_ics,
        meta_version: row.meta_version,
    })
}

// ────────────────────────────────────────────────────────────────────────────
// Owner-side routes
// ────────────────────────────────────────────────────────────────────────────

/// Create a new invitation. Owner-only AND tier-gated (Pro required).
#[utoipa::path(
    post,
    path = "/calendars/{id}/invitations",
    operation_id = "create_invitation",
    tag = "sharing",
    request_body = CreateInvitationRequest,
    responses(
        (status = 201, body = CalendarInvitation),
        (status = 400, description = "Invalid email or self-invite"),
        (status = 402, description = "Owner is on Free tier"),
        (status = 403, description = "Caller is not the owner"),
        (status = 404),
        (status = 409, description = "editor_cap_reached or invite_already_pending"),
        (status = 429, description = "Per-inviter hourly rate limit exceeded"),
    ),
    security(("bearer_auth" = []))
)]
#[post("/calendars/{id}/invitations")]
#[allow(clippy::future_not_send)]
pub async fn create_invitation(
    path: web::Path<i32>,
    body: web::Json<CreateInvitationRequest>,
    claims: Claims,
    user_mapper: web::Data<UserMapper>,
    pool: web::Data<sqlx::PgPool>,
    authz: web::Data<SharingAuthz>,
    svc: web::Data<InvitationService>,
) -> HttpResponse {
    let calendar_id = path.into_inner();

    let actor_id = match claims.user_id() {
        Ok(id) => id,
        Err(e) => return e.error_response(),
    };
    let cal = match load_calendar_any_owner(pool.get_ref(), calendar_id).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };
    if let Err(e) = authz.assert_can(actor_id, &cal, Action::Invite).await {
        return e.error_response();
    }
    let owner = match user_mapper.get_user_from_claims(&claims).await {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };

    match svc
        .send(&cal, &owner.username, &owner.email, body.email.trim())
        .await
    {
        Ok(inv) => HttpResponse::Created().json(inv),
        Err(e) => e.error_response(),
    }
}

/// Revoke a pending invitation. Owner-only.
#[utoipa::path(
    delete,
    path = "/calendars/{id}/invitations/{iid}",
    operation_id = "revoke_invitation",
    tag = "sharing",
    responses(
        (status = 204),
        (status = 403),
        (status = 404),
    ),
    security(("bearer_auth" = []))
)]
#[delete("/calendars/{id}/invitations/{iid}")]
#[allow(clippy::future_not_send)]
pub async fn revoke_invitation(
    path: web::Path<(i32, i64)>,
    claims: Claims,
    pool: web::Data<sqlx::PgPool>,
    authz: web::Data<SharingAuthz>,
    svc: web::Data<InvitationService>,
) -> HttpResponse {
    let (calendar_id, invitation_id) = path.into_inner();

    let actor_id = match claims.user_id() {
        Ok(id) => id,
        Err(e) => return e.error_response(),
    };
    let cal = match load_calendar_any_owner(pool.get_ref(), calendar_id).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };
    if let Err(e) = authz.assert_can(actor_id, &cal, Action::ManageEditors).await {
        return e.error_response();
    }
    match svc.revoke(calendar_id, invitation_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(e) => e.error_response(),
    }
}

/// Resend a pending invitation (mints a fresh token).
#[utoipa::path(
    post,
    path = "/calendars/{id}/invitations/{iid}/resend",
    operation_id = "resend_invitation",
    tag = "sharing",
    responses(
        (status = 200, body = CalendarInvitation),
        (status = 402),
        (status = 403),
        (status = 404),
        (status = 409),
        (status = 429),
    ),
    security(("bearer_auth" = []))
)]
#[post("/calendars/{id}/invitations/{iid}/resend")]
#[allow(clippy::future_not_send)]
pub async fn resend_invitation(
    path: web::Path<(i32, i64)>,
    claims: Claims,
    user_mapper: web::Data<UserMapper>,
    pool: web::Data<sqlx::PgPool>,
    authz: web::Data<SharingAuthz>,
    svc: web::Data<InvitationService>,
) -> HttpResponse {
    let (calendar_id, invitation_id) = path.into_inner();

    let actor_id = match claims.user_id() {
        Ok(id) => id,
        Err(e) => return e.error_response(),
    };
    let cal = match load_calendar_any_owner(pool.get_ref(), calendar_id).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };
    if let Err(e) = authz.assert_can(actor_id, &cal, Action::Invite).await {
        return e.error_response();
    }
    let owner = match user_mapper.get_user_from_claims(&claims).await {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };

    match svc.resend(&cal, &owner.username, invitation_id).await {
        Ok(inv) => HttpResponse::Ok().json(inv),
        Err(e) => e.error_response(),
    }
}

/// List active editors + pending invitations + the configured cap. Owner-only.
#[utoipa::path(
    get,
    path = "/calendars/{id}/editors",
    operation_id = "list_members",
    tag = "sharing",
    responses(
        (status = 200, body = MembersResponse),
        (status = 403),
        (status = 404),
    ),
    security(("bearer_auth" = []))
)]
#[get("/calendars/{id}/editors")]
#[allow(clippy::future_not_send)]
pub async fn list_members(
    path: web::Path<i32>,
    claims: Claims,
    pool: web::Data<sqlx::PgPool>,
    authz: web::Data<SharingAuthz>,
    sharing_cfg: web::Data<SharingConfig>,
    invitations: web::Data<CalendarInvitationMapper>,
) -> HttpResponse {
    let calendar_id = path.into_inner();

    let actor_id = match claims.user_id() {
        Ok(id) => id,
        Err(e) => return e.error_response(),
    };
    let cal = match load_calendar_any_owner(pool.get_ref(), calendar_id).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };
    if let Err(e) = authz.assert_can(actor_id, &cal, Action::ManageEditors).await {
        return e.error_response();
    }

    let editor_rows = match sqlx::query!(
        "SELECT ce.user_id, ce.joined_at, ce.suspended_at, u.username, u.email
         FROM calendar_editors ce
         JOIN users u ON u.id = ce.user_id
         WHERE ce.calendar_id = $1 AND ce.active = true
         ORDER BY ce.joined_at",
        calendar_id,
    )
    .fetch_all(pool.get_ref())
    .await
    {
        Ok(r) => r,
        Err(e) => return Error::Database(e).error_response(),
    };
    let editors: Vec<MemberSummary> = editor_rows
        .into_iter()
        .map(|r| MemberSummary {
            user_id: r.user_id,
            display: r.username,
            email: r.email,
            joined_at: r.joined_at,
            suspended_at: r.suspended_at,
        })
        .collect();

    let pending_invites = match invitations.list_pending_for_calendar(calendar_id).await {
        Ok(p) => p,
        Err(e) => return e.error_response(),
    };
    let pending: Vec<PendingInviteSummary> = pending_invites
        .into_iter()
        .map(|i| PendingInviteSummary {
            id: i.id,
            invitee_email: i.invitee_email,
            sent_at: i.sent_at,
            expires_at: i.expires_at,
        })
        .collect();

    HttpResponse::Ok().json(MembersResponse {
        editors,
        pending,
        editor_cap: sharing_cfg.editor_cap,
    })
}

/// Owner removes an editor (kick). Idempotent.
#[utoipa::path(
    delete,
    path = "/calendars/{id}/editors/{uid}",
    operation_id = "remove_editor",
    tag = "sharing",
    responses((status = 204), (status = 403), (status = 404)),
    security(("bearer_auth" = []))
)]
#[delete("/calendars/{id}/editors/{uid}")]
#[allow(clippy::future_not_send)]
pub async fn remove_editor(
    path: web::Path<(i32, i32)>,
    claims: Claims,
    pool: web::Data<sqlx::PgPool>,
    authz: web::Data<SharingAuthz>,
    editors: web::Data<CalendarEditorMapper>,
) -> HttpResponse {
    let (calendar_id, target_user_id) = path.into_inner();

    let actor_id = match claims.user_id() {
        Ok(id) => id,
        Err(e) => return e.error_response(),
    };
    let cal = match load_calendar_any_owner(pool.get_ref(), calendar_id).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };
    if let Err(e) = authz.assert_can(actor_id, &cal, Action::ManageEditors).await {
        return e.error_response();
    }
    if let Err(e) = editors.remove(calendar_id, target_user_id).await {
        return e.error_response();
    }
    HttpResponse::NoContent().finish()
}

/// Editor leaves a calendar (self-removal). No tier / ownership check —
/// any active editor can drop themselves.
#[utoipa::path(
    delete,
    path = "/calendars/{id}/editors/me",
    operation_id = "leave_calendar",
    tag = "sharing",
    responses((status = 204), (status = 401), (status = 404)),
    security(("bearer_auth" = []))
)]
#[delete("/calendars/{id}/editors/me")]
#[allow(clippy::future_not_send)]
pub async fn leave_calendar(
    path: web::Path<i32>,
    claims: Claims,
    editors: web::Data<CalendarEditorMapper>,
) -> HttpResponse {
    let calendar_id = path.into_inner();
    let actor_id = match claims.user_id() {
        Ok(id) => id,
        Err(e) => return e.error_response(),
    };
    if let Err(e) = editors.remove(calendar_id, actor_id).await {
        return e.error_response();
    }
    HttpResponse::NoContent().finish()
}

// ────────────────────────────────────────────────────────────────────────────
// Invitee-side routes (token-bound)
// ────────────────────────────────────────────────────────────────────────────

/// Public preview for `/invite/:token` landing page. NOT
/// authenticated — callers see only calendar name, owner display, and
/// item count. All token failures collapse to a uniform `400`.
#[utoipa::path(
    get,
    path = "/invitations/{token}",
    operation_id = "preview_invitation",
    tag = "sharing",
    responses(
        (status = 200, body = InvitationPreview),
        (status = 400, description = "Invalid or expired token"),
    ),
)]
#[get("/invitations/{token}")]
#[allow(clippy::future_not_send)]
pub async fn preview_invitation(
    path: web::Path<String>,
    svc: web::Data<InvitationService>,
) -> HttpResponse {
    match svc.preview(path.into_inner().as_str()).await {
        Ok(p) => HttpResponse::Ok().json(p),
        Err(e) => e.error_response(),
    }
}

/// Authenticated accept. Email-bound: caller's email must match the
/// invite's `invitee_email`.
#[utoipa::path(
    post,
    path = "/invitations/{token}/accept",
    operation_id = "accept_invitation",
    tag = "sharing",
    responses(
        (status = 200, body = CalendarInvitation),
        (status = 400, description = "Invalid or expired token"),
        (status = 401),
        (status = 403, description = "Token's invitee email does not match caller"),
    ),
    security(("bearer_auth" = []))
)]
#[post("/invitations/{token}/accept")]
#[allow(clippy::future_not_send)]
pub async fn accept_invitation(
    path: web::Path<String>,
    claims: Claims,
    svc: web::Data<InvitationService>,
) -> HttpResponse {
    let actor_id = match claims.user_id() {
        Ok(id) => id,
        Err(e) => return e.error_response(),
    };
    match svc.accept(actor_id, path.into_inner().as_str()).await {
        Ok(inv) => HttpResponse::Ok().json(inv),
        Err(e) => e.error_response(),
    }
}

/// Authenticated decline.
#[utoipa::path(
    post,
    path = "/invitations/{token}/decline",
    operation_id = "decline_invitation",
    tag = "sharing",
    responses(
        (status = 200, body = EmptyOk),
        (status = 400),
        (status = 401),
        (status = 403),
    ),
    security(("bearer_auth" = []))
)]
#[post("/invitations/{token}/decline")]
#[allow(clippy::future_not_send)]
pub async fn decline_invitation(
    path: web::Path<String>,
    claims: Claims,
    svc: web::Data<InvitationService>,
) -> HttpResponse {
    let actor_id = match claims.user_id() {
        Ok(id) => id,
        Err(e) => return e.error_response(),
    };
    match svc.decline(actor_id, path.into_inner().as_str()).await {
        Ok(()) => HttpResponse::Ok().json(ok_json()),
        Err(e) => e.error_response(),
    }
}
