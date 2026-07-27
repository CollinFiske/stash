const $ = (id) => document.getElementById(id);

function relative(ts) {
  if (!ts) return "never";
  const secs = Math.round((Date.now() - ts) / 1000);
  if (secs < 5) return "just now";
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  return `${Math.floor(mins / 60)}h ago`;
}

async function refresh() {
  const res = await chrome.runtime.sendMessage({ type: "status" }).catch(() => null);

  if (!res || !res.ok) {
    $("dot").className = "dot off";
    $("msg").textContent = "Extension is not responding. Try reloading it.";
    $("msg").className = "msg err";
    return;
  }

  $("windows").textContent = res.lastWindowCount ?? "–";
  $("tabs").textContent = res.lastTabCount ?? "–";

  if (!res.config.token) {
    $("dot").className = "dot off";
    $("msg").innerHTML = "Not paired yet. Open <strong>Settings</strong> and paste the token from Stash.";
    $("msg").className = "msg err";
    return;
  }

  // A push within the last 15s is the same liveness rule the desktop app uses.
  const fresh = res.lastPushAt && Date.now() - res.lastPushAt < 15000;

  if (res.lastError) {
    $("dot").className = "dot off";
    $("msg").textContent = res.lastError;
    $("msg").className = "msg err";
  } else if (fresh) {
    $("dot").className = "dot on";
    $("msg").textContent = `Connected to Stash — last synced ${relative(res.lastPushAt)}.`;
    $("msg").className = "msg";
  } else {
    $("dot").className = "dot";
    $("msg").textContent = `Idle — last synced ${relative(res.lastPushAt)}.`;
    $("msg").className = "msg";
  }
}

$("push").addEventListener("click", async () => {
  $("msg").textContent = "Syncing…";
  $("msg").className = "msg";
  await chrome.runtime.sendMessage({ type: "pushNow" }).catch(() => {});
  setTimeout(refresh, 300);
});

$("options").addEventListener("click", () => chrome.runtime.openOptionsPage());

refresh();
