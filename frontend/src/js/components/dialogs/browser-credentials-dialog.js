import { OCR_PROVIDER_DEFINITIONS, TRANSLATION_PROVIDER_DEFINITION } from "../../provider-config.js";

class BrowserCredentialsDialog extends HTMLElement {
  connectedCallback() {
    if (this.dataset.hydrated === "1") {
      return;
    }
    this.dataset.hydrated = "1";
    const ocrProviderOptions = OCR_PROVIDER_DEFINITIONS.map((provider) => `
      <option value="${provider.id}">${provider.label}</option>
    `).join("");
    const ocrProviderPanels = OCR_PROVIDER_DEFINITIONS.map((provider, index) => `
      <section class="credential-panel credential-provider-panel${index === 0 ? " is-active" : ""}" data-ocr-provider-panel="${provider.id}" role="tabpanel" ${index === 0 ? "" : "hidden"}>
        <div class="credential-card-head">
          <h3>${provider.label}</h3>
          <a class="credential-card-link" href="${provider.docsUrl}" target="_blank" rel="noopener noreferrer">${provider.docsLabel}</a>
        </div>
        <label>
          <span class="developer-label">
            <span>${provider.tokenLabel}</span>
            <button type="button" class="developer-hint" aria-label="${provider.tokenLabel} 说明" data-tooltip="${provider.description}">i</button>
          </span>
          <input id="browser-${provider.id}-token" type="text" autocomplete="off" placeholder="${provider.tokenPlaceholder}" />
        </label>
        <div class="credential-card-actions">
          ${provider.supportsValidation ? `<button id="browser-${provider.id}-validate-btn" type="button" class="secondary">${provider.validationButtonLabel}</button>` : ""}
          <span id="browser-${provider.id}-validation" class="token-inline-status hidden">${provider.validationIdleMessage}</span>
        </div>
      </section>
    `).join("");
    this.innerHTML = `
      <dialog id="browser-credentials-dialog" class="desktop-dialog">
        <form method="dialog" class="desktop-shell">
          <div class="desktop-head">
            <div class="credential-dialog-head">
              <h2>接口设置</h2>
            </div>
            <button id="browser-credentials-close-btn" type="submit" class="dialog-close-btn" aria-label="关闭">×</button>
          </div>
          <div class="desktop-body credential-dialog-body">
            <div class="developer-tabs credential-tabs" role="tablist" aria-label="接口设置">
              <button id="browser-credential-tab-api" type="button" class="developer-tab credential-tab is-active" data-credential-tab="api" role="tab" aria-selected="true">API 设置</button>
              <button id="browser-credential-tab-task" type="button" class="developer-tab credential-tab" data-credential-tab="task" role="tab" aria-selected="false">任务选项</button>
            </div>
            <div class="credential-card-grid credential-panels">
              <section class="credential-panel is-active" data-credential-panel="api" role="tabpanel">
                <div class="credential-card-grid credential-card-grid-compact">
                  <section class="credential-card">
                    <div class="credential-card-head">
                      <h3>OCR</h3>
                    </div>
                    <label>
                      <span class="developer-label">
                        <span>Provider</span>
                      </span>
                      <select id="browser-ocr-provider-select" aria-label="OCR Provider">
                        ${ocrProviderOptions}
                      </select>
                    </label>
                    <div class="credential-provider-panels">
                      ${ocrProviderPanels}
                    </div>
                  </section>

                  <section class="credential-card">
                    <div class="credential-card-head">
                      <h3>${TRANSLATION_PROVIDER_DEFINITION.label}</h3>
                      <a class="credential-card-link" href="${TRANSLATION_PROVIDER_DEFINITION.docsUrl}" target="_blank" rel="noopener noreferrer">${TRANSLATION_PROVIDER_DEFINITION.docsLabel}</a>
                    </div>
                    <label>
                      <span class="developer-label">
                        <span>${TRANSLATION_PROVIDER_DEFINITION.keyLabel}</span>
                      </span>
                      <input id="browser-api-key" type="text" autocomplete="off" placeholder="${TRANSLATION_PROVIDER_DEFINITION.keyPlaceholder}" />
                    </label>
                    <div class="credential-card-actions">
                      <button id="browser-deepseek-validate-btn" type="button" class="secondary">${TRANSLATION_PROVIDER_DEFINITION.validationButtonLabel}</button>
                      <span id="browser-deepseek-validation" class="token-inline-status hidden">${TRANSLATION_PROVIDER_DEFINITION.validationIdleMessage}</span>
                    </div>
                  </section>
                </div>
              </section>

              <section class="credential-card credential-panel" data-credential-panel="task" role="tabpanel" hidden>
                <div class="credential-card-head">
                  <h3>任务选项</h3>
                </div>
                <label>
                  <span class="developer-label">
                    <span>OCR</span>
                  </span>
                  <select id="browser-task-ocr-provider-select" aria-label="任务 OCR Provider">
                    ${ocrProviderOptions}
                  </select>
                </label>
                <label>
                  <span class="developer-label">
                    <span>公式模式</span>
                  </span>
                  <select id="browser-job-math-mode" aria-label="公式模式">
                    <option value="placeholder">占位保护</option>
                    <option value="direct_typst">直出公式</option>
                  </select>
                </label>
              </section>
            </div>
            <div class="actions credential-dialog-actions">
              <button id="browser-credentials-save-btn" type="button">保存</button>
            </div>
          </div>
        </form>
      </dialog>
    `;
  }
}

if (!customElements.get("browser-credentials-dialog")) {
  customElements.define("browser-credentials-dialog", BrowserCredentialsDialog);
}
