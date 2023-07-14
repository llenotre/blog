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

# AWS config
ENV AWS_ACCESS_KEY_ID ""
ENV AWS_SECRET_ACCESS_KEY ""
RUN mkdir -p ~/.aws
RUN echo "[default]\naws_access_key_id = $AWS_ACCESS_KEY_ID\naws_secret_access_key = $AWS_SECRET_ACCESS_KEY" >~/.aws/credentials
RUN echo "[profile default]\nregion = gra\ns3 =\n  endpoint_url = https://s3.gra.io.cloud.ovh.net/\n  signature_version = s3v4\ns3api =\n  endpoint_url = https://s3.gra.io.cloud.ovh.net/" >~/.aws/config

# Setup cron task
COPY backup.sh /usr/bin/backup.sh
CMD chmod +x /usr/bin/backup.sh
RUN echo '0 7 * * 0 bash /usr/bin/backup.sh' >>/etc/crontab

# Run
#CMD ["cron", "-f"]
CMD cat ~/.aws/credentials
