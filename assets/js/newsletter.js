var email = document.getElementById("email");
var button = document.getElementById("subscribe-button");

email.addEventListener("keypress", function(event) {
	if (event.key === "Enter") {
		event.preventDefault();
		button.click();
	}
});

function newsletter_subscribe() {
	if (email.value.length == 0) {
		return;
	}

	var xmlHttp = new XMLHttpRequest();
    xmlHttp.open("POST", "/newsletter/subscribe", false);
    xmlHttp.setRequestHeader("Content-Type", "application/json");

	var payload = JSON.stringify({
		"email": email.value,
	});
    xmlHttp.send(payload);

	if (xmlHttp.status == 200) {
		email.value = "";

		// Indicate success
		button.innerHTML = "<i class=\"fa-solid fa-check\"></i>";
		setTimeout(() => {
			button.innerHTML = "Subscribe";
		}, 3000);
	} else if (xmlHttp.status == 400) {
		alert("Invalid email address");
	} else {
		// TODO get error message from server
		alert("Failed to subscribe to newsletter: HTTP error " + xmlHttp.status);
	}
}
