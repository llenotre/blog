#!/bin/bash

set -e
 
TIMESTAMP=`date --iso-8601=seconds`

mkdir -p backup/
cd backup/

echo "Performing backup $TIMESTAMP..."

mongodump --uri="mongodb://$MONGO_ROOT_USERNAME:$MONGO_ROOT_PASSWORD@mongo:27017" --oplog

echo "Mongodb dump: OK"
 
# Add timestamp to backup
mv dump $TIMESTAMP
tar cf $TIMESTAMP.tar mongodb-$TIMESTAMP
gzip $TIMESTAMP.tar

echo "Compression: OK"
 
# Upload
aws s3 cp $TIMESTAMP.tar.gz s3://maestro/backup/mongodb/

echo "Upload: OK"

# Cleanup
cd ..
rm -rf backup/

echo "Done!"
