let article_id = parseInt(document.getElementById("article-id").value);

let comments_visible = false;
highlight_selected_comment();

let stored_visible = window.localStorage.getItem("comments_visible") === "true";
if (stored_visible && !comments_visible) {
	toggle_comments();
}

// Highlights the select comment
function highlight_selected_comment() {
	let fragment = window.location.hash;
	if (!fragment) {
		return;
	}

	let selected_comment_id = fragment.slice(1);
	if (selected_comment_id.length === 0) {
		return;
	}
	if (!selected_comment_id.startsWith("com-")) {
		return;
	}

	let selected_comment = document.getElementById(selected_comment_id);
	if (!selected_comment) {
		return;
	}

	toggle_comments();
	selected_comment.style.background = '#1abc9c20';
}

/// Copies the given content into clipboard.
function clipboard(id, content) {
	navigator.clipboard.writeText(content);

	let button = document.getElementById(id);
	button.innerHTML = "<i class=\"fa-solid fa-check\"></i>";
	setTimeout(() => {
		button.innerHTML = "<i class=\"fa-solid fa-link\"></i>";
	}, 1000);
}

// Toggles visibility of the comments window.
function toggle_comments() {
	let comments = document.getElementById("comments");
	if (comments_visible) {
		comments.style.display = "none";
	} else {
		comments.style.display = "flex";
	}

	comments_visible = !comments_visible;
	// Keep comments panel open across pages
	window.localStorage.setItem("comments_visible", comments_visible);
}

// Updates the number of characters in the counter
function input(comment_id, action) {
	let comment_content = document.getElementById("comment-" + comment_id + "-" + action + "-content");
	let comment_submit = document.getElementById("comment-" + comment_id + "-" + action + "-submit");
	let comment_len = document.getElementById("comment-" + comment_id + "-" + action + "-len");

	let len = new TextEncoder().encode(comment_content.value).length;
	comment_len.innerHTML = len;
	if (len > 5000) {
		comment_len.style.color = 'red';
	} else {
		comment_len.style.color = 'white';
	}
	comment_submit.disabled = (len <= 0 || len > 5000);
}

// Toggles visibility of the edit editor for the comment with the given ID.
function toggle_edit(comment_id) {
	let edit_div = document.getElementById("editor-" + comment_id + "-edit");
	let reply_div = document.getElementById("editor-" + comment_id + "-reply");
	edit_div.hidden = !edit_div.hidden;
	reply_div.hidden = true;
}

// Toggles visibility of the reply editor for the comment with the given ID.
function toggle_reply(comment_id) {
	let edit_div = document.getElementById("editor-" + comment_id + "-edit");
	let reply_div = document.getElementById("editor-" + comment_id + "-reply");
	edit_div.hidden = true;
	reply_div.hidden = !reply_div.hidden;
}

/// Expands editor on click.
function expand_editor(id) {
	document.getElementById(id).classList.add("expanded");
}

/// Fetches the HTML of a comment with the given ID.
async function fetch_comment(id) {
	return await fetch("/comment/" + id)
		.then(async function(response) {
			let body = await response.text();
			if (response.status === 200) {
				return body;
			} else {
				alert("Failed to fetch comment: " + body);
				return null;
			}
		});
}

// Posts a comment.
async function post(comment_id) {
	let comment_content = document.getElementById("comment-" + comment_id + "-post-content");
	if (comment_content.value.length === 0) {
		return;
	}

    // Post comment
	let headers = new Headers();
	headers.append("Content-Type", "application/json");
	let payload = JSON.stringify({
		"article_id": article_id,
		"reply_to": parseInt(comment_id),
		"content": comment_content.value
	});
	let id = await fetch("/comment", { method: "POST", headers: headers, body: payload })
		.then(async function(response) {
			if (response.status === 200) {
				let json = await response.json();
				return json["id"];
			} else {
				let error = await response.text();
				alert("Failed to post comment: " + error);
				return null;
			}
		});
	if (id == null) {
		return;
	}

	// Get new comment's HTML
	let comment_html = await fetch_comment(id);
	if (comment_html == null) {
		return;
	}

    // Add comment on front-end
	let comments_list;
	if (comment_id == null) {
		comments_list = document.getElementById("comments-list");
	} else {
		comments_list = document.getElementById("comment-" + comment_id + "-replies");
	}
    comments_list.innerHTML += comment_html;

	// Format comment's date
	let com = document.getElementById("com-" + id);
	format_date_long(com.querySelectorAll("[id=date-long]"));

    // Empty text editor
    comment_content.value = "";
    input(comment_id, "post");

    // Update comments count
    let coms_count = document.getElementById("comments-count");
    coms_count.textContent = parseInt(coms_count.textContent) + 1;
}

// Edits the comment with the given ID.
async function edit(comment_id) {
	let comment_content = document.getElementById("comment-" + comment_id + "-edit-content");
	if (comment_content.value.length === 0) {
		return;
	}

    // Update comment
	let headers = new Headers();
    headers.append("Content-Type", "application/json");
	let payload = JSON.stringify({
		"comment_id": parseInt(comment_id),
		"content": comment_content.value
	});
	let response = await fetch("/comment", { method: "PATCH", headers: headers, body: payload });
	if (response.status !== 200) {
		let error = await response.text();
		alert("Failed to edit comment: " + error);
		return;
	}

	// Fetch updated HTML
	let comment_html = await fetch_comment(comment_id);
	if (comment_html == null) {
		return;
	}

    // Update comment on front-end
	let com = document.getElementById("com-" + comment_id);
	com.outerHTML = comment_html;
	format_date_long(com.querySelectorAll("[id=date-long]"));
}

// Deletes the comment with the given ID.
async function del(comment_id) {
	if (!confirm("Are you sure you want to delete this comment?")) {
		return;
	}

	let response = await fetch("/comment/" + comment_id, { method: "DELETE" });
	if (response.status !== 200) {
		let error = await response.text();
		alert("Failed to delete comment: " + error);
		return;
	}

    // Remove comment from front-end
    let com = document.getElementById("com-" + comment_id);
    com.parentNode.removeChild(com);

    // Update comments count
    let coms_count = document.getElementById("comments-count");
    coms_count.textContent = parseInt(coms_count.textContent) - 1;
}
