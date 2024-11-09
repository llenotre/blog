var email = document.getElementById("email");
var button = document.getElementById("subscribe-button");

email.addEventListener("keypress", function(event) {
	if (event.key === "Enter") {
		event.preventDefault();
		button.click();
	}
});

async function newsletter_subscribe() {
	if (email.value.length === 0) {
		return;
	}

	var headers = new Headers();
	headers.append("Content-Type", "application/json");
	var payload = JSON.stringify({
		"email": email.value,
	});
	var [status, msg] = await fetch("https://gateway.maestr.org/newsletter/subscribe", { method: "POST", headers: headers, body: payload })
		.then(async function(response) {
            return [response.status, await response.text()];
		});

	if (status === 200) {
		email.value = "";

		// Indicate success
		button.innerHTML = "<i class=\"fa-solid fa-check\"></i>";
		setTimeout(() => {
			button.innerHTML = "Subscribe";
		}, 3000);
	} else {
		alert("Failed to subscribe to newsletter: " + msg);
	}
}
