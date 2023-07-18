FROM debian:10.13-slim

# Set cron timezone
ENV CRON_TZ=UTC

# Install mongodump
RUN apt-get update
RUN apt-get install -y curl python3-pip cron
RUN curl "https://fastdl.mongodb.org/tools/db/mongodb-database-tools-debian10-x86_64-100.7.3.deb" -o "mongodb-tools.deb"
RUN dpkg -i mongodb-tools.deb
RUN apt-get remove -y curl

# Install AWS cli
RUN pip3 install awscli awscli-plugin-endpoint

# Setup cron task
COPY backup.sh /usr/bin/backup.sh
CMD chmod +x /usr/bin/backup.sh
RUN echo '0 7 * * 0 bash /usr/bin/backup.sh' >>/etc/crontab

# Run
CMD ["cron", "-f"]
