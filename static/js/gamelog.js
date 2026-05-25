// Initialize tooltips for BP death event causes
const tooltipTriggerList = [].slice.call(document.querySelectorAll('[data-bs-toggle="tooltip"]'))
const tooltipList = tooltipTriggerList.map(function (tooltipTriggerEl) {
    return new bootstrap.Tooltip(tooltipTriggerEl)
})

// Event selectors (game only, chat only, everything)
document.getElementById("allevents").addEventListener('change', function () {
    if (this.value) {
        setVisible("#events > .log-chat-entry", true)
        setVisible("#events > .log-entry", true)
    }
})

document.getElementById("gameonly").addEventListener('change', function () {
    if (this.value) {
        setVisible("#events > .log-chat-entry", false)
        setVisible("#events > .log-entry", true)
    }
})

document.getElementById("chatonly").addEventListener('change', function () {
    if (this.value) {
        setVisible("#events > .log-chat-entry", true)
        setVisible("#events > .log-entry", false)
    }
})

function setVisible(selector, visible) {
    document.querySelectorAll(selector).forEach(e => e.classList.toggle("d-none", !visible))
}
