use actix_web::{
    body::MessageBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use std::rc::Rc;
use std::time::Instant;

use crate::metrics::names::*;

pub struct HttpMetrics;

impl<S, B> Transform<S, ServiceRequest> for HttpMetrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = HttpMetricsMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(HttpMetricsMiddleware { service: Rc::new(service) })
    }
}

pub struct HttpMetricsMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for HttpMetricsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let method = req.method().as_str().to_owned();
        // match_pattern() returns the route template (e.g. "/calendars/{id}") instead of the
        // raw URI ("/calendars/42"), keeping Prometheus label cardinality bounded.
        let path = req.match_pattern().unwrap_or_else(|| "unmatched".to_owned());
        let start = Instant::now();

        Box::pin(async move {
            let res = service.call(req).await?;
            let status = res.status().as_u16().to_string();
            let elapsed = start.elapsed().as_secs_f64();

            metrics::counter!(
                HTTP_REQUESTS_TOTAL,
                LABEL_METHOD => method.clone(),
                LABEL_PATH => path.clone(),
                LABEL_STATUS => status.clone(),
            )
            .increment(1);

            metrics::histogram!(
                HTTP_REQUEST_DURATION_SECONDS,
                LABEL_METHOD => method,
                LABEL_PATH => path,
                LABEL_STATUS => status,
            )
            .record(elapsed);

            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    #[actix_web::test]
    async fn middleware_records_request_count_and_duration() {
        let snapshotter = crate::test_helpers::shared_snapshotter();

        let app = test::init_service(
            App::new()
                .wrap(HttpMetrics)
                .route("/probe/{id}", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;

        let req = test::TestRequest::get().uri("/probe/42").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let snapshot = snapshotter.snapshot().into_hashmap();
        let names: Vec<_> = snapshot.keys().map(|k| k.key().name().to_string()).collect();
        assert!(
            names.contains(&crate::metrics::names::HTTP_REQUESTS_TOTAL.to_string()),
            "http_requests_total missing; got {names:?}"
        );
        assert!(
            names.contains(&crate::metrics::names::HTTP_REQUEST_DURATION_SECONDS.to_string()),
            "http_request_duration_seconds missing; got {names:?}"
        );

        let labels: Vec<_> = snapshot
            .keys()
            .filter(|k| k.key().name() == crate::metrics::names::HTTP_REQUESTS_TOTAL)
            .flat_map(|k| k.key().labels().collect::<Vec<_>>())
            .map(|l| (l.key().to_string(), l.value().to_string()))
            .collect();
        assert!(
            labels.iter().any(|(k, v)| k == crate::metrics::names::LABEL_PATH && v == "/probe/{id}"),
            "path label should be pattern '/probe/{{id}}' not raw URI; got {labels:?}"
        );
    }
}
