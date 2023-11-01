use lettre::message::{Mailbox, MultiPart};
use std::pin::pin;
use futures_util::stream::StreamExt;
use lettre::transport::smtp::{
	authentication::{Credentials, Mechanism},
	PoolConfig,
};
use lettre::{Message, SmtpTransport, Transport};
use serde::Deserialize;
use std::{fs, iter};
use tracing::info;
use tracing::warn;
use tokio_postgres::NoTls;

#[derive(Deserialize)]
struct Config {
	/// Database connection string.
	db: String,

	/// The address of the sender server.
	host: String,
	/// The login of the account on the server.
	login: String,
	/// The password of the account on the server.
	password: String,
}

// TODO handle errors
#[tokio::main(flavor = "current_thread")]
async fn main() {
	// Read configuration
	let config = fs::read_to_string("config.toml").unwrap();
	let config: Config = toml::from_str(&config).unwrap();

	let subject = ""; // TODO take from arg
	let html = fs::read_to_string("body.html").unwrap();
	let plain = fs::read_to_string("body.txt").unwrap();

	info!("open database connection");
	// Open database connection
	// TODO tls
	let (client, connection) = tokio_postgres::connect(&config.db, NoTls).await.unwrap();
	// TODO re-open on error
	tokio::spawn(async move {
		connection.await.unwrap();
	});

	info!("open SMTP server connection");
	// Connect to SMTP server
	let sender = SmtpTransport::starttls_relay(&config.host)
		.unwrap()
		.credentials(Credentials::new(config.login, config.password))
		.authentication(vec![Mechanism::Login])
		.pool_config(PoolConfig::new().max_size(20))
		.build();

	let from: Mailbox = "llenotre <blog@lenot.re>".parse().unwrap();
	let body = MultiPart::alternative_plain_html(html, plain);

	info!("fetch recipients");
	let mut emails = pin!(client.query_raw("SELECT DISTINCT email FROM newsletter_subscriber WHERE email IS NOT NULL", iter::empty::<i32>()).await.unwrap());

	info!("send emails");
	let mut count = 0;
	while let Some(email) = emails.next().await {
		let email: String = email.unwrap().get(0);
		let Ok(to) = email.parse::<Mailbox>() else {
			warn!(email, "invalid recipient");
			continue;
		};

		let message = Message::builder()
			.from(from.clone())
			.to(to)
			.subject(subject)
			.multipart(body.clone())
			.unwrap();
		sender.send(&message).unwrap();
		count += 1;
	}

	info!(%count, "done");
}
