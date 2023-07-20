var article_id = document.getElementById("article-id").value;

highlight_selected_comment();

// Highlights the select comment
function highlight_selected_comment() {
	var fragment = window.location.hash;
	if (!fragment) {
		return;
	}

	var selected_comment_id = fragment.slice(1);
	if (selected_comment_id.length == 0) {
		return;
	}
	if (!selected_comment_id.startsWith("com-")) {
		return;
	}

	var selected_comment = document.getElementById(selected_comment_id);
	if (!selected_comment) {
		return;
	}

	toggle_comments();
	selected_comment.style.background = '#1abc9c20';
}

/// Copies the given content into clipboard.
function clipboard(id, content) {
	navigator.clipboard.writeText(content);

	var button = document.getElementById(id);
	button.innerHTML = "<i class=\"fa-solid fa-check\"></i>";
	setTimeout(() => {
		button.innerHTML = "<i class=\"fa-solid fa-link\"></i>";
	}, 1000);
}

var comments_visible = false;

// Toggles visibility of the comments window.
function toggle_comments() {
	var comments = document.getElementById("comments");
	if (comments_visible) {
		comments.style.display = "none";
	} else {
		comments.style.display = "flex";
	}
	comments_visible = !comments_visible;
}

// Toggles visibility of a reactions selector.
function toggle_reactions(id) {
	var selector = document.getElementById(id);
	selector.hidden = !selector.hidden;
}

// Updates the number of characters in the counter
function input(comment_id, action) {
	var comment_content = document.getElementById("comment-" + comment_id + "-" + action + "-content");
	var comment_submit = document.getElementById("comment-" + comment_id + "-" + action + "-submit");
	var comment_len = document.getElementById("comment-" + comment_id + "-" + action + "-len");

	var len = new TextEncoder().encode(comment_content.value).length;
	comment_len.innerHTML = len;
	comment_submit.disabled = (len <= 0 || len > 5000);
}

// Toggles visibility of the edit editor for the comment with the given ID.
function toggle_edit(comment_id) {
	var edit_div = document.getElementById("editor-" + comment_id + "-edit");
	var reply_div = document.getElementById("editor-" + comment_id + "-reply");
	edit_div.hidden = !edit_div.hidden;
	reply_div.hidden = true;
}

// Toggles visibility of the reply editor for the comment with the given ID.
function toggle_reply(comment_id) {
	var edit_div = document.getElementById("editor-" + comment_id + "-edit");
	var reply_div = document.getElementById("editor-" + comment_id + "-reply");
	edit_div.hidden = true;
	reply_div.hidden = !reply_div.hidden;
}

/// Expands editor on click.
function expand_editor(id) {
	document.getElementById(id).classList.add("expanded");
}

// Posts a comment.
async function post(comment_id) {
	var comment_content = document.getElementById("comment-" + comment_id + "-post-content");
	if (comment_content.value.length == 0) {
		return;
	}

    // Post comment
	var headers = new Headers();
	headers.append("Content-Type", "application/json");
	var payload = JSON.stringify({
		"article_id": article_id,
		"response_to": comment_id,
		"content": comment_content.value
	});
	var id = await fetch("/comment", { method: "POST", headers: headers, body: payload })
		.then(async function(response) {
			if (response.status == 200) {
				var json = await response.json();
				return json["id"];
			} else {
				var error = await response.text();
				alert("Failed to post comment:" + error);
				return null;
			}
		});
	// Get new comment's HTML
	var comment_html = await fetch("/comment/" + id)
		.then(async function(response) {
			var body = await response.text();
			if (response.status == 200) {
				return body;
			} else {
				alert("Failed to post comment:" + body);
				return null;
			}
		});

    // Add comment on front-end
	var comments_list;
	if (comment_id == null) {
		comments_list = document.getElementById("comments-list");
	} else {
		comments_list = document.getElementById("comment-" + comment_id + "-replies");
	}
    comments_list.innerHTML += comment_html;

	// Format comment's date
	var com = document.getElementById("com-" + id);
	format_date_long(com.querySelector("#date-long"));

    // Empty text editor
    comment_content.value = "";
    input(comment_id, "post");

    // Update comments count
    var coms_count = document.getElementById("comments-count");
    coms_count.textContent = parseInt(coms_count.textContent) + 1;
}

// Edits the comment with the given ID.
async function edit(comment_id) {
	var comment_content = document.getElementById("comment-" + comment_id + "-edit-content");
	if (comment_content.value.length == 0) {
		return;
	}

    // Update comment
	var headers = new Headers();
    headers.append("Content-Type", "application/json");
	var payload = JSON.stringify({
		"comment_id": comment_id,
		"content": comment_content.value
	});
	var response = await fetch("/comment", { method: "PATCH", headers: headers, body: payload });
	if (response.status != 200) {
		var error = await response.text();
		alert("Failed to edit comment" + error);
		return;
	}

    // Update comment on front-end
    // TODO
}

// Deletes the comment with the given ID.
async function del(comment_id) {
	if (!confirm("Are you sure you want to delete this comment?")) {
		return;
	}

	var response = await fetch("/comment/" + comment_id, { method: "DELETE" });
	if (response.status != 200) {
		var error = await response.text();
		alert("Failed to delete comment" + error);
		return;
	}

    // Remove comment from front-end
    var com = document.getElementById("com-" + comment_id);
    com.parentNode.removeChild(com);

    // Update comments count
    var coms_count = document.getElementById("comments-count");
    coms_count.textContent -= 1;
}
