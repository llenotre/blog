#!/bin/bash

set -e
 
TIMESTAMP=`date --iso-8601=seconds`

mkdir -p backup/
cd backup/

echo "Perform backup $TIMESTAMP..."

PGPASSWORD="$(cat /run/secrets/dbpass)" pg_dump -U blog -F c blog | gzip >$TIMESTAMP.tar.gz

echo "Dump: OK"

# Upload
aws s3 cp $TIMESTAMP.tar.gz s3://blog-backups/

echo "Upload: OK"

# Cleanup
cd ..
rm -rf backup/

echo "Done!"
