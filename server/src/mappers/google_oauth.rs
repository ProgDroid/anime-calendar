use google_oauth::AsyncClient;

use crate::ServerResult;

#[derive(Clone)]
pub struct GoogleOauth {
    client: AsyncClient,
}

pub struct GoogleUser {
    pub id: String,
    pub full_name: String,
    pub email: String,
    pub avatar_url: String,
}

impl GoogleOauth {
    #[must_use]
    pub fn new(client_id: &str) -> Self {
        Self {
            client: AsyncClient::new(client_id),
        }
    }

    /// # Errors
    /// Fails if token is invalid
    pub async fn validate_id_token(&self, id_token: &str) -> ServerResult<GoogleUser> {
        let payload = match self.client.validate_id_token(id_token).await {
            Ok(payload) => payload,
            Err(e) => return Err(e.into()),
        };

        Ok(GoogleUser {
            id: payload.sub,
            full_name: payload.name.unwrap_or_default(),
            email: payload.email.unwrap_or_default(),
            avatar_url: payload.picture.unwrap_or_default(),
        })
    }
}
