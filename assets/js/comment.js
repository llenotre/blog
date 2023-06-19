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

// Toggles visibility of the comments window.
function toggle_comments() {
	var comments = document.getElementById("comments");
	comments.hidden = !comments.hidden;
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

	var len = comment_content.value.length;
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
	document.getElementById(id).style.height = "300px";
}

// Posts a comment.
function post(comment_id) {
	var comment_content = document.getElementById("comment-" + comment_id + "-post-content");
	if (comment_content.value.length == 0) {
		return;
	}

	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("POST", "/comment", false);
    xmlHttp.setRequestHeader("Content-Type", "application/json");

	var payload = JSON.stringify({
		"article_id": article_id,
		"response_to": comment_id,

		"content": comment_content.value
	});
    xmlHttp.send(payload);

	if (xmlHttp.status == 200) {
		// Empty text editor
		comment_content.value = "";
		input(comment_id);

		// TODO insert new comment in the list
	} else {
		// TODO get error message from server
		alert("Failed to post comment: HTTP error " + xmlHttp.status);
	}
}

// Edits the comment with the given ID.
function edit(comment_id) {
	var comment_content = document.getElementById("comment-" + comment_id + "-edit-content");
	if (comment_content.value.length == 0) {
		return;
	}

	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("PATCH", "/comment", false);
    xmlHttp.setRequestHeader("Content-Type", "application/json");

	var payload = JSON.stringify({
		"comment_id": comment_id,

		"content": comment_content.value
	});
    xmlHttp.send(payload);

	if (xmlHttp.status == 200) {
		// TODO update comment's content
	} else {
		// TODO get error message from server
		alert("Failed to edit comment: HTTP error " + xmlHttp.status);
	}
}

// Deletes the comment with the given ID.
function del(comment_id) {
	if (!confirm("Are you sure you want to delete this comment?")) {
		return;
	}

	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("DELETE", "/comment/" + comment_id, false);
    xmlHttp.send(null);

	if (xmlHttp.status == 200) {
		// TODO delete comment
	} else {
		// TODO get error message from server
		alert("Failed to delete comment: HTTP error " + xmlHttp.status);
	}
}
