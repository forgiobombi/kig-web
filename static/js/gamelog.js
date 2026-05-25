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

// Player stats popovers
(function initPlayerPopovers() {
    const statsEl = document.getElementById("kig-player-stats")
    if (!statsEl) return

    let statsByPlayer = {}
    try {
        statsByPlayer = JSON.parse(statsEl.textContent || "{}") || {}
    } catch (_) {
        statsByPlayer = {}
    }

    const modeInfo = {
        cai: { label: "Cowboys and Indians", icon: "ri-sword-fill" },
        timv: { label: "Trouble in Mineville", icon: "ri-spy-fill" },
        bp: { label: "BlockParty", icon: "ri-gamepad-fill" },
        grav: { label: "Gravity", icon: "ri-rocket-2-fill" },
        bed: { label: "Bed Wars", icon: "ri-hotel-bed-fill" },
        halloween2023: { label: "Kig-o'-ween", icon: "ri-ghost-fill" },
        halloween2024: { label: "Kig-o'-ween", icon: "ri-ghost-fill" },
        halloween2025: { label: "Kig-o'-ween", icon: "ri-ghost-fill" },
        turf2026: { label: "Turf Wars", icon: "ri-flag-fill" },
    }

    function safeNumber(n) {
        return typeof n === "number" && Number.isFinite(n) ? n : 0
    }

    function formatKD(kills, deaths) {
        if (deaths === 0) return kills > 0 ? `${kills}.00` : "0.00"
        return (kills / deaths).toFixed(2)
    }

    function el(tag, className, text) {
        const node = document.createElement(tag)
        if (className) node.className = className
        if (text !== undefined) node.textContent = text
        return node
    }

    function buildTitle(modeId) {
        const title = el("div", "kig-player-popover-title")

        const brand = el("div", "kig-player-popover-brand")
        const logo = document.createElement("img")
        logo.className = "kig-player-popover-logo"
        logo.alt = "KIG"
        logo.src = "/game-static/favicon.png"
        logo.width = 16
        logo.height = 16
        brand.appendChild(logo)
        brand.appendChild(el("span", "kig-player-popover-brand-text", "KIG Network"))

        const mode = el("div", "kig-player-popover-mode")
        const info = modeInfo[modeId] || { label: modeId || "Game", icon: "ri-gamepad-fill" }
        const icon = el("i", `kig-player-popover-mode-icon ${info.icon}`)
        mode.appendChild(icon)
        mode.appendChild(el("span", "kig-player-popover-mode-text", info.label))

        title.appendChild(brand)
        title.appendChild(mode)
        return title
    }

    function buildContent(playerKey, playerUuid, displayName, modeId) {
        const stats = statsByPlayer[playerKey] || {}
        const kills = safeNumber(stats.kills)
        const deaths = safeNumber(stats.deaths)
        const joins = safeNumber(stats.joins)
        const leaves = safeNumber(stats.leaves)
        const messages = safeNumber(stats.messages)
        const extra = stats.extra && typeof stats.extra === "object" ? stats.extra : {}

        const root = el("div", "kig-player-popover-body")

        const top = el("div", "kig-player-popover-top")
        const skin = document.createElement("img")
        skin.className = "kig-player-popover-skin"
        skin.alt = ""
        skin.width = 56
        skin.height = 56
        skin.loading = "lazy"
        skin.src = `https://crafthead.net/armor/body/${encodeURIComponent(playerUuid)}`

        const ident = el("div", "kig-player-popover-ident")
        ident.appendChild(el("div", "kig-player-popover-name", displayName || playerKey || "Player"))
        ident.appendChild(
            el("div", "kig-player-popover-sub", `K/D ${formatKD(kills, deaths)} • ${kills} K • ${deaths} D`)
        )

        top.appendChild(skin)
        top.appendChild(ident)
        root.appendChild(top)

        const chips = el("div", "kig-player-popover-chips")
        function chip(label, value) {
            const c = el("span", "kig-stat-chip")
            c.appendChild(el("span", "kig-stat-chip-label", label))
            c.appendChild(el("span", "kig-stat-chip-value", String(value)))
            return c
        }

        chips.appendChild(chip("Joins", joins))
        chips.appendChild(chip("Leaves", leaves))
        chips.appendChild(chip("Messages", messages))
        root.appendChild(chips)

        const extraEntries = Object.entries(extra)
            .filter(([, v]) => safeNumber(v) > 0)
            .sort((a, b) => safeNumber(b[1]) - safeNumber(a[1]))
            .slice(0, 6)

        if (extraEntries.length > 0) {
            const extraWrap = el("div", "kig-player-popover-extra")
            for (const [k, v] of extraEntries) {
                extraWrap.appendChild(chip(k, safeNumber(v)))
            }
            root.appendChild(extraWrap)
        }

        return root
    }

    document.querySelectorAll("[data-kig-player]").forEach((node) => {
        const playerKey = node.getAttribute("data-kig-player") || ""
        const playerUuid = node.getAttribute("data-kig-uuid") || ""
        const modeId = node.getAttribute("data-kig-mode") || ""
        const displayName = (node.querySelector(".kig-player-name") || {}).textContent || ""

        if (!playerKey || !playerUuid) return

        new bootstrap.Popover(node, {
            container: "body",
            trigger: "hover focus",
            placement: "auto",
            html: true,
            customClass: "kig-player-popover",
            title: () => buildTitle(modeId),
            content: () => buildContent(playerKey, playerUuid, displayName, modeId),
            delay: { show: 80, hide: 120 },
        })
    })
})()
