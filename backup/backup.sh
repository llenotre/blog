#!/bin/bash

set -e
 
TIMESTAMP=`date --iso-8601=seconds`

mkdir -p backup/
cd backup/

echo "Performing backup $TIMESTAMP..."

mongodump --uri="mongodb://$MONGO_ROOT_USERNAME:$MONGO_ROOT_PASSWORD@mongo:27017"

echo "Mongodb dump: OK"
 
# Add timestamp to backup
tar --force-local -cf $TIMESTAMP.tar dump
gzip $TIMESTAMP.tar

echo "Compression: OK"
 
# Upload
aws s3 cp $TIMESTAMP.tar.gz s3://maestro/backup/mongodb/

echo "Upload: OK"

# Cleanup
cd ..
rm -rf backup/

echo "Done!"
