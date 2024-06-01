The web service of my blog.



## Configuration

Configuration is done through environment variables. The following variables are being used:
- `BLOG_PORT`: The port on which the HTTP server listens
- `BLOG_DB`: The URL of the postgres database
- `BLOG_SESSION_SECRET_KEY`: The secret key for session cookies
- `BLOG_DISCORD_INVITE`: The URL of the invitation to the Discord server
- `BLOG_GITHUB_CLIENT_ID`: The client ID of the Github app
- `BLOG_GITHUB_CLIENT_SECRET`: The client secret of the Github app



## Setup

The following steps are required:
- Download the GeoIP2 database and place it at `analytics/geoip.mmdb`
- Download the [uaparser regexes file](https://raw.githubusercontent.com/ua-parser/uap-core/master/regexes.yaml) and place it at `analytics/uaparser.yaml`
