FROM debian:10.13-slim

# Set cron timezone
ENV CRON_TZ=UTC

RUN apt-get update
RUN apt-get install -y cron mongo-tools curl unzip

# Install AWS cli
RUN curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
RUN unzip awscliv2.zip
RUN ./aws/install
# Cleanup
RUN rm -rf awscliv2.zip aws/
RUN apt-get remove -y curl unzip
RUN apt-get clean

# Setup cron task
COPY backup.sh /usr/bin/backup.sh
CMD chmod +x /usr/bin/backup.sh
RUN echo '0 7 * * 0 bash /usr/bin/backup.sh' >>/etc/crontab

ENTRYPOINT ["cron"]
