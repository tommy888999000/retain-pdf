import { OCR_PROVIDER_DEFINITIONS, TRANSLATION_PROVIDER_DEFINITION } from "../../provider-config.js";

class DesktopSetupDialog extends HTMLElement {
  connectedCallback() {
    if (this.dataset.hydrated === "1") {
      return;
    }
    this.dataset.hydrated = "1";
    const providerOptions = OCR_PROVIDER_DEFINITIONS.map((provider) => `
      <option value="${provider.id}">${provider.label}</option>
    `).join("");
    const providerPanels = OCR_PROVIDER_DEFINITIONS.map((provider, index) => `
      <label data-setup-provider-panel="${provider.id}" ${index === 0 ? "" : "hidden"}>
        <span>${provider.tokenLabel}</span>
        <input
          id="setup-${provider.id}-token"
          type="text"
          autocomplete="off"
          placeholder="${provider.tokenPlaceholder}"
        />
      </label>
    `).join("");
    this.innerHTML = `
      <dialog id="desktop-setup-dialog" class="desktop-dialog">
        <form method="dialog" class="desktop-shell">
          <div class="desktop-head">
            <h2>首次配置</h2>
            <button id="desktop-setup-close-btn" type="submit" class="dialog-close-btn" aria-label="关闭">×</button>
          </div>
          <div class="desktop-body">
            <p class="muted">首次使用前，请先选择 OCR Provider，并填写对应凭证与 ${TRANSLATION_PROVIDER_DEFINITION.keyLabel}。</p>
            <div class="grid two">
              <label>
                <span>OCR Provider</span>
                <select id="setup-ocr-provider">
                  ${providerOptions}
                </select>
              </label>
              <label>
                <span>${TRANSLATION_PROVIDER_DEFINITION.keyLabel}</span>
                <input id="setup-model-api-key" type="text" autocomplete="off" placeholder="${TRANSLATION_PROVIDER_DEFINITION.keyPlaceholder}" />
              </label>
              ${providerPanels}
            </div>
            <div id="desktop-setup-error" class="upload-status hidden"></div>
            <div class="actions">
              <button id="desktop-setup-save-btn" type="button">保存并启动</button>
            </div>
          </div>
        </form>
      </dialog>
    `;
  }
}

if (!customElements.get("desktop-setup-dialog")) {
  customElements.define("desktop-setup-dialog", DesktopSetupDialog);
}
