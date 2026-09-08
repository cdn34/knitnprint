FROM node:24-alpine@sha256:e67514e5d0f6c46656005e1b693b2ec9d52e80b641307de684d4a015ba7a4eaf AS build

WORKDIR /app

COPY package.json package-lock.json ./
COPY apps/admin/package.json apps/admin/package.json
COPY apps/storefront/package.json apps/storefront/package.json
COPY packages/api-client/package.json packages/api-client/package.json
RUN npm ci

COPY apps/storefront apps/storefront
COPY packages/api-client packages/api-client
RUN npm run build --workspace=@knitnprint/storefront

FROM node:24-alpine@sha256:e67514e5d0f6c46656005e1b693b2ec9d52e80b641307de684d4a015ba7a4eaf AS runtime

ENV NODE_ENV=production \
    HOST=0.0.0.0 \
    PORT=3000

WORKDIR /app

RUN apk upgrade --no-cache \
    && addgroup --system --gid 10001 knitnprint \
    && adduser --system --uid 10001 --ingroup knitnprint --disabled-password knitnprint

COPY --from=build --chown=knitnprint:knitnprint /app/apps/storefront/.output ./.output

USER knitnprint

EXPOSE 3000

CMD ["node", ".output/server/index.mjs"]
