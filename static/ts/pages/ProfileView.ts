const error: HTMLElement = document.getElementById("error")!;
const success: HTMLElement = document.getElementById("success")!;

const puffer = document.getElementById("puffer-url")!.innerText;

// edit about
const edit_form: HTMLFormElement | null = document.getElementById(
    "edit-about"
) as HTMLFormElement | null;

if (edit_form) {
    // edit user about
    edit_form.addEventListener("submit", async (e) => {
        e.preventDefault();
        const res = await fetch(edit_form.getAttribute("data-endpoint")!, {
            method: "POST",
            body: JSON.stringify({
                about: edit_form.about.value,
            }),
            headers: {
                "Content-Type": "application/json",
            },
        });

        const json = await res.json();

        if (json.success === false) {
            error.style.display = "block";
            error.innerHTML = `<div class="mdnote-title">${json.message}</div>`;
        } else {
            edit_form.reset();
            window.location.href = "?";
        }
    });
}

// follow
const follow_button: HTMLButtonElement | null = document.getElementById(
    "follow-user"
) as HTMLButtonElement | null;

if (follow_button) {
    // follow user
    follow_button.addEventListener("click", async (e) => {
        e.preventDefault();
        const res = await fetch(follow_button.getAttribute("data-endpoint")!, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
        });

        const json = await res.json();

        if (json.success === false) {
            error.style.display = "block";
            error.innerHTML = `<div class="mdnote-title">${json.message}</div>`;
        } else {
            success.style.display = "block";
            success.innerHTML = `<div class="mdnote-title">${json.message}</div>`;
        }
    });
}

// mail
const mail_button: HTMLButtonElement | null = document.getElementById(
    "mail-user"
) as HTMLButtonElement | null;

if (mail_button) {
    // mail user
    mail_button.addEventListener("click", async (e) => {
        e.preventDefault();
        const res = await fetch(mail_button.getAttribute("data-endpoint")!, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
        });

        const json = await res.json();

        if (json.success === false) {
            error.style.display = "block";
            error.innerHTML = `<div class="mdnote-title">${json.message}</div>`;
        } else {
            window.location.href = `${puffer}${json.payload.name}`;
        }
    });
}

// default export
export default {};
