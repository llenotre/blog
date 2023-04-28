var article_id = document.getElementById("article-id");

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

// TODO doc
function post(response_to) {
	var comment_content = document.getElementById("comment-" + response_to + "-content");

	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("POST", "/comment", false);
    xmlHttp.setRequestHeader("Content-Type", "application/json");

	var payload = JSON.stringify({
		"article_id": article_id.value,
		"response_to": response_to,

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

// TODO doc
function edit(comment_id) {
	var edit_editor_div = document.getElementById("edit-editor-" + comment_id);
	var reply_editor_div = document.getElementById("reply-editor-" + comment_id);

	edit_editor_div.hidden = !edit_editor_div.hidden;
	reply_editor_div.hidden = true;
}

// TODO doc
function del(comment_id) {
	// TODO add delete confirm

	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("DELETE", "/comment/" + comment_id, false);
    xmlHttp.send(null);

	location.reload();
}

// TODO doc
function reply(comment_id) {
	var edit_editor_div = document.getElementById("edit-editor-" + comment_id);
	var reply_editor_div = document.getElementById("reply-editor-" + comment_id);

	edit_editor_div.hidden = true;
	reply_editor_div.hidden = !reply_editor_div.hidden;
}
