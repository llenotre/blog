var article_id = document.getElementById("article-id");

// The comment to reply to.
var reply_to = null;

var comments_visible = false;

// Toggles visibility of the comments window.
function toggle_comments() {
	var fixed_buttons = document.getElementById("fixed-buttons");
	var sections = document.getElementsByClassName("article-section");
	var comments = document.getElementById("comments");

	if (comments_visible) {
		fixed_buttons.style.left = "30%";
		fixed_buttons.style.transform = "translateX(-100%)";
		for (let i = 0; i < sections.length; i++) {
			sections[i].style.paddingLeft = "30%";
			sections[i].style.paddingRight = "30%";
		}

		comments.style.display = "none";
	} else {
		fixed_buttons.style.left = "0";
		fixed_buttons.style.transform = "translateX(0)";
		for (let i = 0; i < sections.length; i++) {
			sections[i].style.paddingLeft = "150px";
			sections[i].style.paddingRight = "45vw";
		}

		comments.style.display = "flex";
	}

	comments_visible = !comments_visible;
}

// Updates the number of characters in the counter
function input(comment_id) {
	var comment_content = document.getElementById("comment-" + comment_id + "-content");
	var comment_submit = document.getElementById("comment-" + comment_id + "-submit");
	var comment_len = document.getElementById("comment-" + comment_id + "-len");

	var len = comment_content.value.length;
	comment_len.innerHTML = len;
	comment_submit.disabled = (len > 10000);
}

// Posts a comment.
function post(_) {
	var comment_content = document.getElementById("comment-null-content");
	if (comment_content.value.length == 0) {
		return;
	}

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
		alert("Failed to post comment: HTTP error " + xmlHttp.status);
	}
}

// Toggles visibility of the edit editor for the comment with the given ID.
function toggle_edit(comment_id) {
	var editor_div = document.getElementById("editor-" + comment_id);
	editor_div.hidden = !editor_div.hidden;
}

// Edits the comment with the given ID.
function edit(comment_id) {
	var comment_content = document.getElementById("comment-" + comment_id + "-content");
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
		location.reload()
	} else {
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
		location.reload()
	} else {
		alert("Failed to delete comment: HTTP error " + xmlHttp.status);
	}
}

/// Sets the comment to be replied to.
function set_reply(comment_id) {
	reply_to = comment_id;

	var reply_to_elem = document.getElementById("reply-to");
	reply_to_elem.innerHTML = "Reply to comment <a href=\"#" + reply_to + "\">#" + reply_to + "</a>";
	reply_to_elem.hidden = false;
	reply_to_elem.scrollIntoView();
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

/// Expands editor on click.
function expand_editor(id) {
	document.getElementById("comment-" + id + "-content").style.height = "300px";
}
