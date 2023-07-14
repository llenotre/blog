#!/bin/bash
 
TIMESTAMP=`date --iso-8601=seconds`
DBNAME="blog"

mkdir -p backup/
cd backup/

# Lock DB
mongo admin --eval "printjson(db.fsyncLock())"
mongodump --username $MONGO_ROOT_USERNAME --password $MONGO_ROOT_PASSWORD -d $DBNAME mongodb://mongo:27017
# Unlock DB
mongo admin --eval "printjson(db.fsyncUnlock())"
 
# Add timestamp to backup
mv dump $TIMESTAMP
tar cf $TIMESTAMP.tar mongodb-$TIMESTAMP
gzip $TIMESTAMP.tar
 
# Upload
aws s3 cp $TIMESTAMP.tar.gz s3://maestro/backup/mongodb/$DBNAME
 

# Cleanup
cd ..
rm -rf backup/
