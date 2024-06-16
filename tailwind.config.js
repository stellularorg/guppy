/** @type {import('tailwindcss').Config} */
module.exports = {
    content: ["./templates/**/*.html"],
    darkMode: "class",
    theme: {
        extend: {
            colors: {
                brand: "hsl(214, 100%, 75%)",
                "brand-low": "hsl(214, 100%, 70%)",
            },
        },
    },
    plugins: [],
};
