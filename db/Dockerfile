FROM postgres:16.1-bullseye

USER root

# Init schema
COPY schema.sql /
COPY init.sh /docker-entrypoint-initdb.d/init.sh

# Set cron timezone
ENV CRON_TZ=UTC

# Install utilities
RUN apt-get update
RUN apt-get install -y python3-pip cron
RUN pip3 install awscli awscli-plugin-endpoint

# Setup cron task
COPY backup.sh /usr/bin/backup.sh
RUN chmod +x /usr/bin/backup.sh
RUN echo '0 7 * * 0 root /usr/bin/backup.sh' >>/etc/crontab