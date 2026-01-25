(() => {
  const DROPZONE_SELECTOR = "#files-table";
  const DROPZONE_BASE_CLASSES = [
    "rounded-3",
    "bg-white",
    "position-relative",
    "overflow-hidden",
  ];
  const DROPZONE_ACTIVE_CLASSES = ["shadow", "bg-primary-subtle"];
  const DROPZONE_DIM_CLASSES = ["opacity-50", "bg-body"];
  const OVERLAY_CLASSES = [
    "position-absolute",
    "top-50",
    "start-50",
    "translate-middle",
    "d-flex",
    "flex-column",
    "align-items-center",
    "gap-2",
    "text-primary",
    "fw-semibold",
    "user-select-none",
    "text-center",
    "pe-none",
    "z-2",
  ];

  const prevent = (event) => {
    event.preventDefault();
    event.stopPropagation();
  };

  const assignFilesToInput = (fileInput, files) => {
    if (!files.length) {
      return false;
    }

    if (typeof DataTransfer === "function") {
      const dataTransfer = new DataTransfer();
      files.forEach((file) => dataTransfer.items.add(file));
      fileInput.files = dataTransfer.files;
      return true;
    }

    try {
      fileInput.files = files;
      return true;
    } catch (error) {
      console.warn(
        "Drag-and-drop upload is not supported in this browser.",
        error,
      );
      return false;
    }
  };

  const triggerUpload = (form) => {
    if (!form) {
      return;
    }

    if (window.htmx && typeof window.htmx.trigger === "function") {
      window.htmx.trigger(form, "submit");
      return;
    }

    if (typeof form.requestSubmit === "function") {
      form.requestSubmit();
      return;
    }

    form.submit();
  };

  const wireDropzone = (root) => {
    const table = root.querySelector(DROPZONE_SELECTOR);
    const fileInput = root.querySelector('input[type="file"]#file');
    const form = fileInput ? fileInput.closest("form") : null;

    if (
      !table ||
      !fileInput ||
      !form ||
      table.dataset.dropzoneInitialized === "true"
    ) {
      return;
    }

    table.dataset.dropzoneInitialized = "true";
    DROPZONE_BASE_CLASSES.forEach((cls) => table.classList.add(cls));

    const overlay = document.createElement("div");
    overlay.classList.add(...OVERLAY_CLASSES, "d-none");
    overlay.innerHTML = `
      <i class="bi bi-cloud-upload display-5" aria-hidden="true"></i>
      <span class="lead">Drop files to upload</span>
    `;
    table.appendChild(overlay);

    const tableBodies = Array.from(table.tBodies ?? []);

    const showOverlay = () => overlay.classList.remove("d-none");
    const hideOverlay = () => overlay.classList.add("d-none");

    let dragCounter = 0;

    const highlight = (event) => {
      prevent(event);
      dragCounter += 1;
      DROPZONE_ACTIVE_CLASSES.forEach((cls) => table.classList.add(cls));
      tableBodies.forEach((body) =>
        DROPZONE_DIM_CLASSES.forEach((cls) => body.classList.add(cls)),
      );
      showOverlay();
    };

    const unhighlight = (event) => {
      prevent(event);
      dragCounter = Math.max(dragCounter - 1, 0);

      if (dragCounter === 0) {
        DROPZONE_ACTIVE_CLASSES.forEach((cls) => table.classList.remove(cls));
        tableBodies.forEach((body) =>
          DROPZONE_DIM_CLASSES.forEach((cls) => body.classList.remove(cls)),
        );
        hideOverlay();
      }
    };

    const resetHighlight = () => {
      dragCounter = 0;
      DROPZONE_ACTIVE_CLASSES.forEach((cls) => table.classList.remove(cls));
      tableBodies.forEach((body) =>
        DROPZONE_DIM_CLASSES.forEach((cls) => body.classList.remove(cls)),
      );
      hideOverlay();
    };

    const handleDrop = (event) => {
      prevent(event);
      resetHighlight();

      const droppedFiles = Array.from(event.dataTransfer?.files ?? []);
      if (!assignFilesToInput(fileInput, droppedFiles)) {
        return;
      }

      triggerUpload(form);
    };

    table.addEventListener("dragenter", highlight);
    table.addEventListener("dragover", highlight);
    table.addEventListener("dragleave", unhighlight);
    table.addEventListener("dragend", unhighlight);
    table.addEventListener("drop", handleDrop);

    window.addEventListener("dragover", resetHighlight);
    window.addEventListener("dragleave", resetHighlight);
    window.addEventListener("dragend", resetHighlight);
    const events = ["dragenter", "dragover", "dragleave", "dragend", "drop"];
    attachDebugLogging(table, events, "table");
    attachDebugLogging(window, events, "window");
  };

  const init = (target) => {
    const root = target instanceof HTMLElement ? target : document;
    wireDropzone(root);
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => init(document));
  } else {
    init(document);
  }

  if (window.htmx && typeof window.htmx.onLoad === "function") {
    window.htmx.onLoad(init);
  }
})();
