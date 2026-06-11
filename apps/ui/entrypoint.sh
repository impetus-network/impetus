#!/bin/sh
set -e

if [ -n "$INFISICAL_CLIENT_ID" ] && [ -n "$INFISICAL_CLIENT_SECRET" ]; then
  INFISICAL_TOKEN=$(infisical login \
    --method=universal-auth \
    --client-id="$INFISICAL_CLIENT_ID" \
    --client-secret="$INFISICAL_CLIENT_SECRET" \
    --domain "${INFISICAL_DOMAIN:-https://secret-manager.impetus.network/api}" \
    --plain --silent)

  exec infisical run \
    --token "$INFISICAL_TOKEN" \
    --projectId "$INFISICAL_PROJECT_ID" \
    --env "${INFISICAL_ENV:-dev}" \
    --domain "${INFISICAL_DOMAIN:-https://secret-manager.impetus.network/api}" \
    -- "$@"
fi

exec "$@"
