<template>
  <div class="page-container">
    <h1 class="page-h1">FAQ</h1>

    <div class="faq-list">
      <div class="faq-item">
        <div class="faq-q">How do I save a workspace?</div>
        <div class="faq-a">
          Click <strong>+ Take Snapshot</strong> in the header. Stash scans all open
          user-facing windows, then asks you to give the snapshot a name. Hit
          Save and it's stored locally, no account needed.
        </div>
      </div>

      <div class="faq-item">
        <div class="faq-q">How does restoring a workspace work?</div>
        <div class="faq-a">
          Click <strong>Launch</strong> on any saved workspace or template. Stash opens
          each app in sequence and attempts to reposition it on your current screen.
          Positions are stored as fractions, so layouts scale across different monitor sizes.
        </div>
      </div>

      <div class="faq-item">
        <div class="faq-q">What are Project Templates?</div>
        <div class="faq-a">
          Templates are hand-crafted blueprints for a set of apps in a specific layout.
          Unlike snapshots they're not tied to a particular moment in time, define
          them once and reuse them whenever you switch contexts.
        </div>
      </div>

      <div class="faq-item">
        <div class="faq-q">Why isn't an app showing up in my snapshot?</div>
        <div class="faq-a">
          Stash filters out background and system processes, capturing only
          user-facing windows. Fully minimized windows or apps with no visible
          frame may also be skipped depending on your OS.
        </div>
      </div>

      <div class="faq-item">
        <div class="faq-q">How does active workspace detection work?</div>
        <div class="faq-a">
          Every few seconds Stash checks which apps are running. If every app in a
          workspace or template is open, that card turns yellow and a
          <strong>Close Apps</strong> button appears. Close the workspace and the
          card returns to normal.
        </div>
      </div>

      <div class="faq-item">
        <div class="faq-q">Can I edit a saved workspace or template?</div>
        <div class="faq-a">
          Yes, click the <strong>⋮</strong> button on any card and choose
          <strong>Edit</strong>. For workspaces you can rename them and remove
          individual apps. For templates you can rename, add, or remove apps freely.
        </div>
      </div>

      <div class="faq-item">
        <div class="faq-q">Where is my data stored?</div>
        <div class="faq-a">
          Everything is stored locally in a <code>workspaces.json</code> file inside
          your app data directory. Nothing leaves your machine.
        </div>
      </div>
    </div>
  </div>
</template>
