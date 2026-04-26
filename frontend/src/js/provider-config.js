export const DEFAULT_OCR_PROVIDER = "paddle";

export const OCR_PROVIDER_DEFINITIONS = [
  {
    id: "paddle",
    label: "PaddleOCR",
    description: "用于 PaddleOCR 在线 OCR 解析。",
    tokenField: "paddle_token",
    runtimeConfigKey: "paddleToken",
    tokenLabel: "Paddle Access Token",
    tokenPlaceholder: "填写 Paddle Access Token",
    validationButtonLabel: "检测 Paddle",
    validationIdleMessage: "可检测 Paddle Access Token 是否可用。",
    validationMissingMessage: "请先填写 Paddle Access Token。",
    validationUnavailableMessage: "",
    docsUrl: "https://aistudio.baidu.com/account/accessToken",
    docsLabel: "获取 Token",
    supportsValidation: true,
  },
  {
    id: "mineru",
    label: "MinerU",
    description: "用于 OCR 解析和版面识别。",
    tokenField: "mineru_token",
    runtimeConfigKey: "mineruToken",
    tokenLabel: "MinerU Token",
    tokenPlaceholder: "填写 MinerU Token",
    validationButtonLabel: "检测 MinerU",
    validationIdleMessage: "保存前会自动检测 MinerU Token。",
    validationMissingMessage: "请先填写 MinerU Token。",
    validationUnavailableMessage: "",
    docsUrl: "https://mineru.net/apiManage/docs?openApplyModal=true",
    docsLabel: "获取 Token",
    supportsValidation: true,
  },
];

export const TRANSLATION_PROVIDER_DEFINITION = {
  id: "deepseek",
  label: "DeepSeek",
  keyLabel: "DeepSeek Key",
  keyPlaceholder: "填写 DeepSeek API Key",
  description: "用于正文翻译和模型调用。",
  docsUrl: "https://platform.deepseek.com/api_keys",
  docsLabel: "获取 Key",
  validationButtonLabel: "检测 DeepSeek",
  validationIdleMessage: "可检测 DeepSeek 接口是否连通。",
  validationMissingMessage: "请先填写 DeepSeek Key。",
  validationSuccessMessage: "DeepSeek 接口连接成功。",
  validationNetworkMessage: "DeepSeek 接口检测失败，请检查网络或浏览器跨域限制。",
  validationUnauthorizedMessage: "DeepSeek Key 无效或已过期。",
};

export function normalizeOcrProvider(value) {
  const provider = `${value || ""}`.trim().toLowerCase();
  return OCR_PROVIDER_DEFINITIONS.some((item) => item.id === provider) ? provider : DEFAULT_OCR_PROVIDER;
}

export function getOcrProviderDefinition(provider) {
  return OCR_PROVIDER_DEFINITIONS.find((item) => item.id === normalizeOcrProvider(provider)) || OCR_PROVIDER_DEFINITIONS[0];
}
