var article_id = document.getElementById("article-id");
var comment_content = document.getElementById("comment-content");
var comment_submit = document.getElementById("comment-submit");
var comment_len = document.getElementById("comment-len");

comment_content.addEventListener("input", input);
comment_submit.addEventListener("click", post);

function input(event) {
	var len = comment_content.value.length;
	comment_len.innerHTML = len;
	comment_submit.disabled = (len > 10000);
}

function post(event) {
	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("POST", "/comment", false);
    xmlHttp.setRequestHeader("Content-Type", "application/json");

	var payload = JSON.stringify({
		"article_id": article_id.value,
		// TODO "response_to": "",

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

function edit(comment_id) {
	// TODO
}

// TODO add delete confirm
function del(comment_id) {
	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("DELETE", "/comment/" + comment_id, false);
    xmlHttp.send(null);

	location.reload()
}

function reply(comment_id) {
	// TODO
}
