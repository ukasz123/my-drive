const attachDebugLogging = (element, events, label) => {
  if (!element || !Array.isArray(events)) {
    return;
  }

  const effectiveLabel = label ?? "my-drive";

  const logEvent = (event) => {
    console.log(`[${effectiveLabel}] ${event.type}`);
  };

  events.forEach((eventName) => {
    element.addEventListener(eventName, logEvent);
  });
};
