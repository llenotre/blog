use lettre::{Message, SmtpTransport, Transport};
use lettre::message::MultiPart;
use lettre::transport::smtp::{
    authentication::{Credentials, Mechanism},
    PoolConfig
};

#[serde(Deserialize)]
struct Config {
    // TODO db conn string

    /// The address of the sender server.
    host: String,
    /// The login of the account on the server.
    login: String,
    /// The password of the account on the server.
    password: String,
}

fn main() {
    // Read configuration
    let config = fs::read_to_string("config.toml").unwrap();
    let config: Config = toml::from_str(&config).unwrap();

    let sender = SmtpTransport::starttls_relay(config.host)
        .unwrap()
        .credentials(Credentials::new(
            config.login,
            config.password,
        ))
        .authentication(vec![Mechanism::Login])
        .pool_config(PoolConfig::new().max_size(20))
        .build();

    // TODO for each recipient
    let message = Message::builder()
        .from("llenotre <blog@lenot.re>".parse().unwrap())
        // TODO To
        .subject("Hello")
        .multipart(MultiPart::alternative_plain_html("hello".to_owned(), "<p style=\"color: red\">hello</p>".to_owned()))
        .unwrap();
    sender.send(&message).unwrap();
}