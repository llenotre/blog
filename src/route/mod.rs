use crate::GlobalData;
use actix_session::Session;
use actix_web::http::header::ContentType;
use actix_web::{error, get, web, HttpResponse, Responder};
use crate::service::article::Article;
use crate::service::user::User;

pub mod article;
pub mod comment;
pub mod file;
pub mod newsletter;
pub mod user;

#[get("/")]
pub async fn root(
	data: web::Data<GlobalData>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let db = data.get_database();
	let admin = User::check_admin(&db, &session).await.map_err(|e| {
		tracing::error!(error = %e, "mongodb: user");
		error::ErrorInternalServerError("")
	})?;

	// Get articles
	let articles = Article::list(&db).await.map_err(|e| {
		tracing::error!(error = %e, "mongodb: articles");
		error::ErrorInternalServerError("")
	})?;

	// Produce articles HTML
	let mut articles_html = String::new();
	for article in articles {
		let content = article.get_content(&db).await.map_err(|e| {
			tracing::error!(error = %e, "mongodb: article content");
			error::ErrorInternalServerError("")
		})?;
		if !admin && !content.public {
			continue;
		}

		let post_date = if let Some(post_date) = article.post_date {
			post_date.to_rfc3339()
		} else {
			"not posted yet".to_string()
		};

		let mut tags = vec![];

		if admin {
			let pub_tag = if content.public { "Public" } else { "Private" };
			tags.push(pub_tag);
		}

		if content.sponsor {
			tags.push("<i>Sponsors early access</i>&nbsp;❤️");
		}
		if !content.tags.is_empty() {
			tags.extend(content.tags.split(','));
		}

		let tags_html: String = tags
			.into_iter()
			.map(|s| format!(r#"<li class="tag">{s}</li>"#))
			.collect();

		articles_html.push_str(&format!(
			r#"<a href="{article_path}">
				<div class="article-element">
					<img class="article-cover" src="{article_cover_url}"></img>
					<div class="article-element-content">
						<h3>{article_title}</h3>

						<ul class="tags">
							<li><h6 style="color: gray;"><span id="date">{post_date}</span></h6></li>
							{tags_html}
						</ul>

						<p>
							{article_desc}
						</p>
					</div>
				</div>
			</a>"#,
			article_cover_url = content.cover_url,
			article_path = content.get_path(),
			article_title = content.title,
			article_desc = content.desc,
		));
	}

	let html = include_str!("../../pages/index.html");
	let html = html.replace("{discord.invite}", &data.discord_invite);
	let html = html.replace("{articles}", &articles_html);

	Ok(HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html))
}

#[get("/bio")]
pub async fn bio() -> impl Responder {
	let html = include_str!("../../pages/bio.html");
	HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html)
}

#[get("/legal")]
pub async fn legal() -> impl Responder {
	let html = include_str!("../../pages/legal.html");
	HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html)
}

#[get("/robots.txt")]
pub async fn robots() -> impl Responder {
	r#"User-agent: *
Allow: /
Sitemap: https://blog.lenot.re/sitemap.xml"#
}

#[get("/sitemap.xml")]
pub async fn sitemap(data: web::Data<GlobalData>) -> actix_web::Result<impl Responder> {
	let mut urls = vec![];

	urls.push(("/".to_owned(), None));
	urls.push(("/bio".to_owned(), None));
	urls.push(("/legal".to_owned(), None));

	let db = data.get_database();
	let articles = Article::list(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	for a in articles {
		let content = a
			.get_content(&db)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?;

		urls.push((content.get_url(), Some(content.edit_date)));
	}

	let urls: String = urls
		.into_iter()
		.map(|(url, date)| match date {
			Some(date) => {
				let date = date.format("%Y-%m-%d");
				format!("\t\t<url><loc>{url}</loc><lastmod>{date}</lastmod></url>")
			}

			None => format!("\t\t<url><loc>{url}</loc></url>"),
		})
		.collect();

	let body = format!(
		r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
	{urls}
</urlset>"#
	);

	Ok(HttpResponse::Ok()
		.content_type(ContentType::xml())
		.body(body))
}

#[get("/rss")]
pub async fn rss(data: web::Data<GlobalData>) -> actix_web::Result<impl Responder> {
	let db = data.get_database();
	let articles = Article::list(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let mut items_str = String::new();
	for a in articles {
		let Some(ref post_date) = a.post_date else {
			continue;
		};
		let post_date = post_date.to_rfc2822();

		let content = a
			.get_content(&db)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?;
		let url = content.get_url();

		items_str.push_str(&format!(
			"<item><guid>{url}</guid><title>{title}</title><link>{url}</link><pubDate>{post_date}</pubDate><description>{desc}</description><author>llenotre</author></item>",
			title = content.title,
			desc = content.desc
		));
	}

	let body = format!(
		r#"<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom"><channel><atom:link href="https://blog.lenot.re/rss" rel="self" type="application/rss+xml" /><title>Luc Lenôtre</title><link>https:/blog.lenot.re/</link><description>A blog about writing an operating system from scratch in Rust.</description>{items_str}</channel></rss>"#
	);

	Ok(HttpResponse::Ok()
		.content_type(ContentType::xml())
		.body(body))
}
