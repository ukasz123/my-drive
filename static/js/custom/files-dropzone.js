(() => {
  const DROP_ACTIVE_CLASS = "files-dropzone--active";
  const DROPZONE_BASE_CLASS = "files-dropzone";
  const DROPZONE_STYLE_ID = "files-dropzone-styles";
  const DROPZONE_SELECTOR = "#files-table";

  const prevent = (event) => {
    event.preventDefault();
    event.stopPropagation();
  };

  const ensureStyles = () => {
    if (document.getElementById(DROPZONE_STYLE_ID)) {
      return;
    }

    const style = document.createElement("style");
    style.id = DROPZONE_STYLE_ID;
    style.textContent = `
${DROPZONE_SELECTOR}.${DROPZONE_BASE_CLASS} {
  transition: box-shadow 0.2s ease, background-color 0.2s ease;
}
${DROPZONE_SELECTOR}.${DROP_ACTIVE_CLASS} {
  background-color: rgba(13, 110, 253, 0.08);
  box-shadow: 0 0 0 2px rgba(13, 110, 253, 0.5);
}
${DROPZONE_SELECTOR}.${DROP_ACTIVE_CLASS} tbody {
  opacity: 0.5;
}
`;
    document.head.appendChild(style);
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
    ensureStyles();
    table.classList.add(DROPZONE_BASE_CLASS);

    let dragCounter = 0;

    const highlight = (event) => {
      prevent(event);
      dragCounter += 1;
      table.classList.add(DROP_ACTIVE_CLASS);
    };

    const unhighlight = (event) => {
      prevent(event);
      dragCounter = Math.max(dragCounter - 1, 0);

      if (dragCounter === 0) {
        table.classList.remove(DROP_ACTIVE_CLASS);
      }
    };

    const resetHighlight = () => {
      dragCounter = 0;
      table.classList.remove(DROP_ACTIVE_CLASS);
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
