const error: HTMLElement = document.getElementById("error")!;
const success: HTMLElement = document.getElementById("success")!;

// edit about
const edit_form: HTMLFormElement | null = document.getElementById(
    "edit-about",
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
    "follow-user",
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

// post activity
const compose_form: HTMLFormElement | null = document.getElementById(
    "compose_activity",
) as HTMLFormElement | null;

if (compose_form) {
    // post new activity
    compose_form.addEventListener("submit", async (e) => {
        e.preventDefault();
        const res = await fetch(compose_form.getAttribute("data-endpoint")!, {
            method: "POST",
            body: JSON.stringify({
                content: compose_form.content.value,
                author: "",
                reply: (compose_form.reply || { value: "" }).value,
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
            if (compose_form.getAttribute("data-reply")) {
                window.location.reload();
                return;
            }
            
            window.location.href = `${window.location.href}/activity/${json.payload.id}`;
        }
    });
}

// post favorites
(globalThis as any).favorite_post = async (id: string) => {
    const res = await fetch(`/api/v1/activity/${id}/favorite`, {
        method: "POST",
    });

    const json = await res.json();

    if (json.success === false) {
        error.style.display = "block";
        error.innerHTML = `<div class="mdnote-title">${json.message}</div>`;
    } else {
        success.style.display = "block";
        success.innerHTML = `<div class="mdnote-title">${json.message}</div>`;
    }
};

// default export
export default {};
