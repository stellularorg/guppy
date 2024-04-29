const error: HTMLElement = document.getElementById("error")!;
const success: HTMLElement = document.getElementById("success")!;
const forms: HTMLElement = document.getElementById("forms")!;
const switch_button: HTMLElement = document.getElementById("switch-button")!;

const register_form: HTMLFormElement | null = document.getElementById(
    "register-user"
) as HTMLFormElement | null;

const login_form: HTMLFormElement | null = document.getElementById(
    "login-user"
) as HTMLFormElement | null;

const login_st_form: HTMLFormElement | null = document.getElementById(
    "login-user-st"
) as HTMLFormElement | null;

const callback = document.getElementById("callback")!.innerText;

if (register_form) {
    // register
    register_form.addEventListener("submit", async (e) => {
        e.preventDefault();
        const res = await fetch("/api/auth/register", {
            method: "POST",
            body: JSON.stringify({
                username: register_form.username.value,
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
            success.style.display = "block";
            success.innerHTML = `<p>Account created! You can login using this code:</p>

            <p class="card border round flex justify-center align-center">${json.message}</p>

            <p><b>Do not lose it!</b> This code is required for you to sign into your account, <b>it cannot be reset!</b></p>
            
            <hr />
            <a href="${callback}?uid=${json.message}" class="button round theme:primary">Continue</a>`;
            forms.style.display = "none";
        }
    });
} else if (login_form) {
    // login
    login_form.addEventListener("submit", async (e) => {
        e.preventDefault();
        const res = await fetch("/api/auth/login", {
            method: "POST",
            body: JSON.stringify({
                uid: login_form.uid.value,
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
            success.style.display = "block";
            success.innerHTML = `<p>Successfully logged into account.</p>
                
                <hr />
                <a href="${callback}?uid=${json.message}" class="button round theme:primary">Continue</a>`;
            forms.style.display = "none";

            if (switch_button) {
                switch_button.remove();
            }
        }
    });
} else if (login_st_form) {
    // login (secondary token)
    login_st_form.addEventListener("submit", async (e) => {
        e.preventDefault();
        const res = await fetch("/api/auth/login-st", {
            method: "POST",
            body: JSON.stringify({
                uid: login_st_form.uid.value,
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
            success.style.display = "block";
            success.innerHTML = `<p>Successfully logged into account.</p>
                
                <hr />
                <a href="${callback}?uid=${json.message}" class="button round theme:primary">Continue</a>`;
            forms.style.display = "none";

            if (switch_button) {
                switch_button.remove();
            }
        }
    });
}

// default export
export default {};
