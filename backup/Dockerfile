FROM debian:10.13-slim

# Set cron timezone
ENV CRON_TZ=UTC

# Install mongodump
RUN apt-get update
RUN apt-get install -y curl unzip cron
RUN curl "https://fastdl.mongodb.org/tools/db/mongodb-database-tools-debian10-x86_64-100.7.3.deb" -o "mongodb-tools.deb"
RUN dpkg -i mongodb-tools.deb

# Install AWS cli
RUN curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscli.zip"
RUN unzip awscli.zip
RUN ./aws/install
# Cleanup
RUN rm -rf awscli.zip aws/
RUN apt-get remove -y curl unzip
RUN apt-get clean

# Setup cron task
COPY backup.sh /usr/bin/backup.sh
CMD chmod +x /usr/bin/backup.sh
RUN echo '0 7 * * 0 bash /usr/bin/backup.sh' >>/etc/crontab

# Run
#CMD ["cron", "-f"]
CMD ["bash", "/usr/bin/backup.sh"]
