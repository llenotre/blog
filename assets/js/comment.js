var article_id = document.getElementById("article-id");

// The comment to reply to.
var reply_to = null;

// Updates the number of characters in the counter
function input(comment_id) {
	var comment_content = document.getElementById("comment-" + comment_id + "-content");
	var comment_submit = document.getElementById("comment-" + comment_id + "-submit");
	var comment_len = document.getElementById("comment-" + comment_id + "-len");

	var len = comment_content.value.length;
	comment_len.innerHTML = len;
	comment_submit.disabled = (len > 10000);
}

// Returns the preview for the given markdown
function get_preview(markdown) {
	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("GET", "/comment/preview", false);
    xmlHttp.send(markdown);

	return xmlHttp.responseText;
}

// Posts a comment.
function post(_) {
	var comment_content = document.getElementById("comment-null-content");

	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("POST", "/comment", false);
    xmlHttp.setRequestHeader("Content-Type", "application/json");

	var payload = JSON.stringify({
		"article_id": article_id.value,
		"response_to": reply_to,

		"content": comment_content.value
	});
    xmlHttp.send(payload);

	if (xmlHttp.status == 200) {
		comment_content.value = "";
		location.reload()
	} else {
		// TODO show error
	}
}

// Toggles visibility of the edit editor for the comment with the given ID.
function toggle_edit(comment_id) {
	var editor_div = document.getElementById("editor-" + comment_id);
	editor_div.hidden = !editor_div.hidden;
}

// Edits the comment with the given ID.
function edit(comment_id) {
	// TODO
}

// TODO doc
function del(comment_id) {
	// TODO add delete confirm

	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("DELETE", "/comment/" + comment_id, false);
    xmlHttp.send(null);

	location.reload();
}

/// Sets the comment to be replied to.
function set_reply(comment_id) {
	reply_to = comment_id;

	var reply_to_elem = document.getElementById("reply-to");
	// TODO onclick, scroll to comment
	reply_to_elem.innerHTML = "Reply to comment <a href=\"#\">#" + reply_to + "</a>";
	reply_to_elem.hidden = false;
	reply_to_elem.scrollIntoView();
}
