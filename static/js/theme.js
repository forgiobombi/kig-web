let storedTheme = null
try {
    storedTheme = JSON.parse(localStorage.getItem("kig-theme") || "null")
} catch (_) {
    storedTheme = null
}

const stylesheet = document.getElementById("bootstrap-dark")
const toggler = document.getElementById("theme-toggler")
let darkTheme = false

setDarkTheme(!!(storedTheme && storedTheme.dark))

function toggleDarkTheme() {
    setDarkTheme(!darkTheme)
}

function setDarkTheme(dark) {
    darkTheme = dark
    stylesheet.setAttribute("href", dark ? "/game-static/css/bootstrap-dark.min.css" : "")
    document.body.setAttribute("data-theme", dark ? "dark" : "") // For other CSS files
    localStorage.setItem("kig-theme", JSON.stringify({ dark }))
    toggler.textContent = dark ? "Dark" : "Light"
}
