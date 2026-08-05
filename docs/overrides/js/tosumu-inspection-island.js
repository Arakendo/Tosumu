const inspectionLab = document.querySelector("[data-tosumu-inspection-lab]");

if (inspectionLab) {
  void setupInspectionLab(inspectionLab);
}

async function setupInspectionLab(inspectionLab) {
  const input = inspectionLab.querySelector("[data-tosumu-inspection-file]");
  const run = inspectionLab.querySelector("[data-tosumu-inspection-run]");
  const fixtures = [...inspectionLab.querySelectorAll("[data-tosumu-inspection-fixture]")];
  const status = inspectionLab.querySelector("[data-tosumu-inspection-status]");
  const output = inspectionLab.querySelector("[data-tosumu-inspection-output]");
  const terminalCanvas = inspectionLab.querySelector("[data-tosumu-ratatui-terminal-canvas]");
  const terminalInput = inspectionLab.querySelector("[data-tosumu-ratatui-terminal-input]");
  const terminalStatus = inspectionLab.querySelector("[data-tosumu-ratatui-terminal-status]");
  const terminalSection = terminalCanvas?.closest(".tosumu-ratatui-terminal");
  let terminalInputActive = false;
  const setStatus = (message) => {
    status.textContent = message;
  };

  try {
    // MkDocs loads site scripts as classic scripts. Keep provider-neutral UI
    // helpers as a dynamic module so their exports never become global state.
    const contract = await import("./tosumu-inspection-island-contract.js");
    const module = await import("./inspection-wasm/tosumu_inspection_wasm.js");
    await module.default();

    const render = (json) => {
      output.textContent = contract.renderInspectionResponse(json);
    };

    const setBusy = (busy) => {
      for (const fixture of fixtures) {
        fixture.disabled = busy;
      }
      run.disabled = busy || !contract.selectedFileState(input.files[0]).canInspect;
    };

    const inspectBytes = (bytes, source) => {
      const json = module.inspect_uploaded_bytes(bytes);
      const response = JSON.parse(json);
      render(json);
      const terminalSnapshot = JSON.parse(module.start_ratatui_terminal(bytes, 84, 22));
      renderRatatuiProjection(terminalCanvas, terminalSnapshot);
      terminalStatus.textContent = "Browser-safe Ratatui terminal ready. Click the terminal, type HELP, STATUS, or CLEAR, then press Enter.";
      const outcome = response.outcome === "failure"
        ? "explicit boundary rejection"
        : "header observation";
      setStatus(`${source}: ${outcome}. Unavailable sections remain explicit contract outcomes.`);
    };

    const interactTerminal = (event) => {
      try {
        const snapshot = JSON.parse(module.interact_ratatui_terminal(event, 84, 22));
        renderRatatuiProjection(terminalCanvas, snapshot);
        terminalStatus.textContent = "Rust/WASM owns this prompt, transcript, history, and command outcome. TQL remains unavailable for raw uploaded bytes.";
      } catch (error) {
        terminalStatus.textContent = `Ratatui terminal input did not apply: ${String(error)}`;
      }
    };

    const focusTerminalInput = () => {
      terminalInputActive = true;
      terminalInput?.focus({ preventScroll: true });
      terminalStatus.textContent = "Terminal input is active. Rust/WASM owns the prompt; Enter submits HELP, STATUS, or CLEAR.";
    };

    // The textarea is a browser-native keyboard and IME transport only. It is
    // cleared after every input event; Rust/WASM retains the prompt, history,
    // transcript, and command outcomes.
    terminalInput?.addEventListener("input", (event) => {
      const inputValue = event.currentTarget.value;
      event.currentTarget.value = "";
      if (!inputValue) {
        return;
      }
      interactTerminal(`text:${inputValue}`);
      terminalStatus.textContent = "Rust/WASM redrew the prompt from the forwarded text input. Enter submits.";
    });

    terminalInput?.addEventListener("paste", (event) => {
      const text = event.clipboardData?.getData("text");
      if (!text) {
        return;
      }
      event.preventDefault();
      interactTerminal(`text:${text}`);
      terminalStatus.textContent = "Rust/WASM redrew the prompt from pasted text. Enter submits.";
    });

    // MkDocs registers global shortcuts for search. Once the user has selected
    // this terminal, consume only terminal-shaped input before it reaches those
    // document-level handlers. The DOM never becomes the command-state owner.
    const terminalKeyEvent = (event) => {
      if (!terminalInputActive) {
        return;
      }
      const special = {
        Backspace: "key:backspace",
        Enter: "key:enter",
        ArrowUp: "key:arrow-up",
        ArrowDown: "key:arrow-down",
        PageUp: "key:page-up",
        PageDown: "key:page-down",
        Home: "key:home",
        End: "key:end",
        Escape: "key:escape",
      }[event.key];
      const printable = event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey;
      if (!special && !printable) {
        return;
      }
      event.stopImmediatePropagation();
      if (event.type === "keydown") {
        if (special) {
          event.preventDefault();
          interactTerminal(special);
          terminalStatus.textContent = "Rust/WASM updated the terminal session.";
        }
      }
    };
    document.addEventListener("keydown", terminalKeyEvent, true);
    document.addEventListener("keypress", terminalKeyEvent, true);
    document.addEventListener("keyup", terminalKeyEvent, true);

    document.addEventListener("pointerdown", (event) => {
      if (terminalSection && !terminalSection.contains(event.target)) {
        terminalInputActive = false;
      }
    }, true);

    terminalSection?.addEventListener("pointerdown", (event) => {
      event.preventDefault();
      focusTerminalInput();
    }, true);
    terminalCanvas?.addEventListener("wheel", (event) => {
      event.preventDefault();
      interactTerminal(event.deltaY < 0 ? "key:scroll-up" : "key:scroll-down");
    }, { passive: false });

    input.addEventListener("change", () => {
      const [file] = input.files;
      const state = contract.selectedFileState(file);
      run.disabled = !state.canInspect;
      setStatus(state.message);
    });

    for (const fixture of fixtures) {
      fixture.disabled = false;
    }
    setStatus(contract.bundledFixtureState().message);
    const inspectFixture = async (fixture) => {
      const fixtureState = contract.fixtureButtonState(fixture);
      setBusy(true);
      setStatus(`Fetching ${fixtureState.label} (${fixtureState.expectation}) into the Tosumu inspection boundary.`);
      try {
        const response = await fetch(fixtureState.url);
        if (!response.ok) {
          throw new Error(`fixture request returned ${response.status}`);
        }
        const bytes = new Uint8Array(await response.arrayBuffer());
        if (bytes.byteLength > contract.INSPECTION_BYTE_LIMIT) {
          throw new Error("bundled fixture exceeds the browser inspection limit");
        }
        inspectBytes(bytes, fixtureState.label);
      } catch (error) {
        output.textContent = "The bundled fixture could not be loaded. Upload inspection remains available.";
        terminalStatus.textContent = "Ratatui command session is unavailable because the reviewed fixture could not be loaded.";
        setStatus(`Bundled fixture inspection failed: ${String(error)}`);
      } finally {
        setBusy(false);
      }
    };

    for (const fixture of fixtures) {
      fixture.addEventListener("click", () => {
        void inspectFixture(fixture);
      });
    }
    run.addEventListener("click", async () => {
      const [file] = input.files;
      if (!contract.selectedFileState(file).canInspect) {
        return;
      }
      setBusy(true);
      setStatus("Reading selected bytes into the Tosumu inspection boundary.");
      try {
        inspectBytes(new Uint8Array(await file.arrayBuffer()), "Uploaded byte");
      } catch (error) {
        output.textContent = "WASM inspection adapter failed to return an observation.";
        setStatus(`Inspection failed: ${String(error)}`);
      } finally {
        setBusy(false);
      }
    });

    // The reviewed fresh-store case makes both provider views visible without
    // requiring visitors to own or upload a Tosumu database file.
    if (fixtures[0]) {
      void inspectFixture(fixtures[0]);
    }
  } catch (error) {
    setStatus(`WASM inspection did not start: ${String(error)}. The static boundary disclosure above remains authoritative.`);
  }
}

function renderRatatuiProjection(canvas, snapshot) {
  if (!canvas || !snapshot || !Number.isInteger(snapshot.width) || !Number.isInteger(snapshot.height)) {
    return;
  }

  const context = canvas.getContext("2d");
  if (!context) {
    return;
  }
  const width = canvas.width;
  const height = canvas.height;
  const cellWidth = width / snapshot.width;
  const cellHeight = height / snapshot.height;
  canvas.setAttribute(
    "aria-label",
    `Interactive Ratatui TestBackend viewport: ${snapshot.width} by ${snapshot.height} cells from the current Tosumu inspection response. Focus it and use arrow keys, Page Up or Down, Home, End, or the mouse wheel to navigate provider-local history.`,
  );
  context.fillStyle = "#071013";
  context.fillRect(0, 0, width, height);
  context.font = `${Math.max(9, Math.floor(cellHeight * 0.8))}px ui-monospace, SFMono-Regular, Consolas, monospace`;
  context.textBaseline = "top";
  for (const cell of snapshot.cells) {
    if (!cell.symbol || cell.symbol === " ") {
      continue;
    }
    context.fillStyle = ratatuiColor(cell.foreground);
    context.fillText(cell.symbol, cell.x * cellWidth, cell.y * cellHeight + 1);
  }
}

function ratatuiColor(color) {
  const palette = {
    Cyan: "#74d4e8",
    Green: "#85e6bd",
    Red: "#fa8787",
    DarkGray: "#8b9b9a",
    Reset: "#e7eeee",
  };
  return palette[color] || "#d7e6e4";
}
