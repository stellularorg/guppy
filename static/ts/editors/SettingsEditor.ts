export function user_settings(
    metadata: { [key: string]: any },
    name: string,
    field: HTMLElement,
    _type: "user" | undefined
): void {
    if (_type === undefined) _type = "user";

    const update_form = document.getElementById(
        "update-form"
    ) as HTMLFormElement;

    const add_field = document.getElementById("add_field") as HTMLButtonElement;

    let current_property: string = "";
    let option_render: string = "";

    // handlers
    (window as any).change_current_property = (e: any) => {
        const selected = e.target.options[
            e.target.selectedIndex
        ] as HTMLOptionElement;

        if (selected) {
            current_property = selected.value;

            // USER ONLY - secondary token
            if (_type === "user" && current_property === "secondary_token") {
                option_render = `<button class="button round theme:primary" onclick="window.send_token_refresh_request();">Refresh Token</button>`;

                (window as any).send_token_refresh_request = async () => {
                    const res = await fetch(
                        `/api/auth/users/${name}/secondary-token`,
                        {
                            method: "POST",
                        }
                    );

                    const res_ = await res.json();
                    alert(res_.payload);
                };

                options = build_options(metadata, current_property);
                render_user_settings_fields(field, options, option_render); // rerender
                return;
            }

            // ...
            let meta_value = metadata[current_property];
            if (typeof meta_value === "string" || meta_value === null) {
                const use =
                    current_property === "about" ||
                    current_property === "page_template"
                        ? "textarea"
                        : "input";
                option_render = `<${use} 
                    type="text" 
                    name="${current_property}" 
                    placeholder="${current_property}" 
                    value="${use === "input" ? meta_value || "" : ""}" 
                    required 
                    oninput="window.user_settings_field_input(event);" 
                    class="round mobile:max"
                    style="width: 60%;"
                ${
                    use === "textarea"
                        ? `>${meta_value || ""}</textarea>`
                        : "/>"
                }`;

                (window as any).user_settings_field_input = (e: any) => {
                    metadata[current_property] = e.target.value;
                };
            }
        }

        options = build_options(metadata, current_property);
        render_user_settings_fields(field, options, option_render); // rerender
    };

    // ...
    let options = build_options(metadata, current_property);
    render_user_settings_fields(field, options, option_render);

    // handle submit
    update_form.addEventListener("submit", async (e) => {
        e.preventDefault();

        // user
        const res = await fetch(`/api/auth/users/${name}/update`, {
            method: "POST",
            body: JSON.stringify(metadata),
            headers: {
                "Content-Type": "application/json",
            },
        });

        const json = await res.json();

        if (json.success === false) {
            return alert(json.message);
        } else {
            window.location.reload();
        }
    });

    // handle add field
    add_field.addEventListener("click", () => {
        const name = prompt("Enter field name:");
        if (!name) return;

        metadata[name] = "unknown";
        options = build_options(metadata, current_property);
        render_user_settings_fields(field, options, option_render);
    });
}

function build_options(
    metadata: { [key: string]: string },
    current_property: string
): string {
    let options: string = ""; // let mut options: String = "";

    for (let option of Object.entries(metadata)) {
        options += `<option value="${option[0]}" ${
            current_property === option[0] ? "selected" : ""
        }>${option[0]}</option>\n`;
    }

    return options;
}

function render_user_settings_fields(
    field: HTMLElement,
    options: string,
    option_render: string
): string {
    field.innerHTML = "";

    // render selector
    field.innerHTML += `<select class="round mobile:max" onchange="window.change_current_property(event);" style="width: 38%;">
        <option value="">Select a field to edit</option>
        ${options}
    </select>${option_render}`;

    // ...
    return "";
}

// default export
export default { user_settings };
