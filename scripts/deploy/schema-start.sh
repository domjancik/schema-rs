#!/bin/sh
set -eu
project=/var/lib/schema
session=$project/.session/session.json
if [ -s "$session" ]; then
 exec /opt/schema/current/schema-rs serve --project "$project" --bind 0.0.0.0:8787 --resume-session "$session"
fi
exec /opt/schema/current/schema-rs serve --project "$project" --bind 0.0.0.0:8787
