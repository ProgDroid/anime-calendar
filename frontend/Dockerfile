FROM node:22-alpine AS builder

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .

# No build-time env vars: public config is delivered at runtime by the backend
# (GET /api/public-config). Keeps the resulting image environment-agnostic so
# the same artefact deploys to staging and production.

RUN npm run build

# ---------------------------------------------------------------------------
FROM nginx:alpine AS runtime

COPY --from=builder /app/dist /usr/share/nginx/html

# nginx:alpine's entrypoint runs envsubst over every *.template in
# /etc/nginx/templates/ and writes the result into /etc/nginx/conf.d/ before
# starting nginx. BACKEND_URL must therefore be set at container start (a
# Cloud Run env var, `docker run -e`, or the compose `environment:` block):
# an unset variable substitutes to an empty string, leaving an invalid
# `proxy_pass /;` that nginx refuses to start with.
COPY nginx.conf.template /etc/nginx/templates/default.conf.template

# Run nginx as non-root (nginx:alpine already sets up the nginx user).
# /etc/nginx/conf.d must be writable by that user as well — it is where the
# entrypoint writes the substituted config, and the entrypoint runs as the
# USER set below.
RUN chown -R nginx:nginx /usr/share/nginx/html && \
    chown -R nginx:nginx /etc/nginx/conf.d && \
    chown -R nginx:nginx /var/cache/nginx && \
    chown -R nginx:nginx /var/log/nginx && \
    touch /var/run/nginx.pid && \
    chown -R nginx:nginx /var/run/nginx.pid

USER nginx

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
