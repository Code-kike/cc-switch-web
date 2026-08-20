/**
 * Codex 预设供应商配置模板
 */
import { ProviderCategory } from "../types";
import type { CodexApiFormat, CodexCatalogModel } from "../types";
import type { PresetTheme } from "./claudeProviderPresets";

export interface CodexProviderPreset {
  name: string;
  nameKey?: string; // i18n key for localized display name
  websiteUrl: string;
  // 第三方供应商可提供单独的获取 API Key 链接
  apiKeyUrl?: string;
  auth: Record<string, any>; // 将写入 ~/.codex/auth.json
  config: string; // 将写入 ~/.codex/config.toml（TOML 字符串）
  isOfficial?: boolean; // 标识是否为官方预设
  isPartner?: boolean; // 标识是否为商业合作伙伴
  partnerPromotionKey?: string; // 合作伙伴促销信息的 i18n key
  category?: ProviderCategory; // 新增：分类
  isCustomTemplate?: boolean; // 标识是否为自定义模板
  // 新增：请求地址候选列表（用于地址管理/测速）
  endpointCandidates?: string[];
  // 新增：视觉主题配置
  theme?: PresetTheme;
  // 图标配置
  icon?: string; // 图标名称
  iconColor?: string; // 图标颜色
  // Codex API 格式
  apiFormat?: CodexApiFormat;
  // 托管账号预设：目前仅 xAI OAuth（本地代理按请求注入 token）
  providerType?: "xai_oauth";
  // OAuth 预设隐藏 API Key，并要求保存前已有可用托管账号
  requiresOAuth?: boolean;
  // Codex 模型目录，保存为 settingsConfig.modelCatalog.models
  modelCatalog?: CodexCatalogModel[];
}

/**
 * 生成第三方供应商的 auth.json
 */
export function generateThirdPartyAuth(apiKey: string): Record<string, any> {
  return {
    OPENAI_API_KEY: apiKey || "",
  };
}

/**
 * 生成第三方供应商的 config.toml
 */
export function generateThirdPartyConfig(
  providerName: string,
  baseUrl: string,
  modelName = "gpt-5.6-sol",
): string {
  // 清理供应商名称，确保符合TOML键名规范
  const cleanProviderName =
    providerName
      .toLowerCase()
      .replace(/[^a-z0-9_]/g, "_")
      .replace(/^_+|_+$/g, "") || "custom";

  return `model_provider = "${cleanProviderName}"
model = "${modelName}"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.${cleanProviderName}]
name = "${cleanProviderName}"
base_url = "${baseUrl}"
wire_api = "responses"
requires_openai_auth = true`;
}

function modelCatalog(
  models: Array<
    | string
    | {
        model: string;
        displayName?: string;
        contextWindow?: number;
        // Native Responses (direct) overrides for the generated
        // model-catalogs.json. Omitted input modalities are inferred by the
        // backend: confirmed text-only models stay text-only; everything else
        // defaults to text+image.
        supportsParallelToolCalls?: boolean;
        inputModalities?: string[];
        baseInstructions?: string;
      }
  >,
): CodexCatalogModel[] {
  return models.map((entry) =>
    typeof entry === "string"
      ? { model: entry }
      : {
          model: entry.model,
          displayName: entry.displayName,
          contextWindow: entry.contextWindow,
          supportsParallelToolCalls: entry.supportsParallelToolCalls,
          inputModalities: entry.inputModalities,
          baseInstructions: entry.baseInstructions,
        },
  );
}

export const codexProviderPresets: CodexProviderPreset[] = [
  {
    name: "OpenAI Official",
    websiteUrl: "https://chatgpt.com/codex",
    isOfficial: true,
    category: "official",
    auth: {},
    config: ``,
    theme: {
      icon: "codex",
      backgroundColor: "#1F2937", // gray-800
      textColor: "#FFFFFF",
    },
    icon: "openai",
    iconColor: "#00A67E",
  },
  {
    name: "Shengsuanyun",
    nameKey: "providerForm.presets.shengsuanyun",
    websiteUrl: "https://www.shengsuanyun.com",
    apiKeyUrl: "https://www.shengsuanyun.com/?from=CH_4HHXMRYF",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "shengsuanyun",
      "https://router.shengsuanyun.com/api/v1",
      "openai/gpt-5.6-sol",
    ),
    category: "aggregator",
    isPartner: true,
    partnerPromotionKey: "shengsuanyun",
    icon: "shengsuanyun",
  },
  {
    name: "PatewayAI",
    websiteUrl: "https://pateway.ai",
    apiKeyUrl: "https://pateway.ai/?ch=etzpm8&aff=WB6M6F67#/",
    category: "third_party",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "patewayai",
      "https://api.pateway.ai/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://api.pateway.ai/v1"],
    isPartner: true,
    partnerPromotionKey: "patewayai",
    icon: "pateway",
  },
  {
    name: "火山 Agent Plan",
    websiteUrl:
      "https://www.volcengine.com/activity/agentplan?ac=MMAP8JTTCAQ2&rc=6J6FV5N2&utm_source=OWO&utm_medium=devrel-1&utm_campaign=hw&utm_term=ccswitch&utm_content=hw",
    apiKeyUrl:
      "https://www.volcengine.com/activity/agentplan?ac=MMAP8JTTCAQ2&rc=6J6FV5N2&utm_source=OWO&utm_medium=devrel-1&utm_campaign=hw&utm_term=ccswitch&utm_content=hw",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "ark_agentplan",
      "https://ark.cn-beijing.volces.com/api/plan/v3",
      "ark-code-latest",
    ),
    // ⚠️ 计费红线（官方 warning）：Agent Plan 必须走 /api/plan/v3；
    // 按量端点 /api/v3 不消耗套餐额度、按量另计费，Coding Plan 的
    // /api/coding/v3 是另一份订阅——两者都绝不能混入候选
    endpointCandidates: ["https://ark.cn-beijing.volces.com/api/plan/v3"],
    // 官方 Codex 文档（volcengine.com/docs/82379/2556056，2026-07 更新）：
    // Agent Plan /api/plan/v3 与 Coding Plan /api/coding/v3 均已支持
    // Responses API（wire_api=responses），无需路由接管转换
    apiFormat: "openai_responses",
    modelCatalog: modelCatalog([
      {
        model: "ark-code-latest",
        displayName: "Ark Code Latest",
        contextWindow: 256000,
      },
    ]),
    category: "cn_official",
    isPartner: true,
    partnerPromotionKey: "volcengine_agentplan",
    icon: "huoshan",
    iconColor: "#3370FF",
  },
  {
    name: "火山 Coding Plan",
    websiteUrl:
      "https://www.volcengine.com/activity/codingplan?ac=MMAP8JTTCAQ2&rc=6J6FV5N2&utm_campaign=hw&utm_content=ccswitch&utm_medium=devrel_tool_web&utm_source=OWO&utm_term=ccswitch",
    apiKeyUrl:
      "https://www.volcengine.com/activity/codingplan?ac=MMAP8JTTCAQ2&rc=6J6FV5N2&utm_campaign=hw&utm_content=ccswitch&utm_medium=devrel_tool_web&utm_source=OWO&utm_term=ccswitch",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "ark_codingplan",
      "https://ark.cn-beijing.volces.com/api/coding/v3",
      "ark-code-latest",
    ),
    // ⚠️ 计费红线（官方 warning）：Coding Plan 必须走 /api/coding/v3；
    // 按量端点 /api/v3 不消耗套餐额度、按量另计费，Agent Plan 的
    // /api/plan/v3 是另一份订阅——两者都绝不能混入候选
    endpointCandidates: ["https://ark.cn-beijing.volces.com/api/coding/v3"],
    // 官方 Codex 文档（volcengine.com/docs/82379/2556056，2026-07 更新）：
    // Coding Plan /api/coding/v3 已支持 Responses API（wire_api=responses），
    // 无需路由接管转换
    apiFormat: "openai_responses",
    modelCatalog: modelCatalog([
      {
        model: "ark-code-latest",
        displayName: "Ark Code Latest",
        contextWindow: 256000,
      },
    ]),
    category: "cn_official",
    isPartner: true,
    partnerPromotionKey: "volcengine_codingplan",
    icon: "huoshan",
    iconColor: "#3370FF",
  },
  {
    name: "Tencent Hunyuan",
    websiteUrl: "https://cloud.tencent.com/product/tokenhub",
    apiKeyUrl: "https://console.cloud.tencent.com/tokenhub/apikey",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "hy3_tokenhub",
      "https://tokenhub.tencentmaas.com/v1",
      "hy3",
    ),
    // The international TokenHub uses region-specific credentials and is not
    // interchangeable with these mainland endpoints.
    endpointCandidates: [
      "https://tokenhub.tencentmaas.com/v1",
      "https://tokenhub.tencentmaas.cn/v1",
    ],
    apiFormat: "openai_responses",
    modelCatalog: modelCatalog([
      {
        model: "hy3",
        displayName: "Hy3",
        contextWindow: 256000,
        inputModalities: ["text"],
      },
      {
        model: "hy3-preview",
        displayName: "Hy3 Preview",
        contextWindow: 256000,
        inputModalities: ["text"],
      },
    ]),
    category: "cn_official",
    icon: "hunyuan",
    iconColor: "#0055E9",
  },
  {
    name: "Azure OpenAI",
    websiteUrl:
      "https://learn.microsoft.com/en-us/azure/ai-foundry/openai/how-to/codex",
    category: "third_party",
    isOfficial: true,
    auth: generateThirdPartyAuth(""),
    config: `model_provider = "azure"
model = "gpt-5.6-sol"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.azure]
name = "Azure OpenAI"
base_url = "https://YOUR_RESOURCE_NAME.openai.azure.com/openai"
env_key = "OPENAI_API_KEY"
query_params = { "api-version" = "2025-04-01-preview" }
wire_api = "responses"
requires_openai_auth = true`,
    endpointCandidates: ["https://YOUR_RESOURCE_NAME.openai.azure.com/openai"],
    theme: {
      icon: "codex",
      backgroundColor: "#0078D4",
      textColor: "#FFFFFF",
    },
    icon: "azure",
    iconColor: "#0078D4",
  },
  {
    name: "DeepSeek",
    websiteUrl: "https://platform.deepseek.com",
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "deepseek",
      "https://api.deepseek.com",
      "deepseek-v4-flash",
    ),
    endpointCandidates: ["https://api.deepseek.com"],
    // DeepSeek's native Responses gateway publishes an official Codex catalog;
    // the backend mirrors its harness and freeform apply_patch declaration.
    apiFormat: "openai_responses",
    modelCatalog: modelCatalog([
      {
        model: "deepseek-v4-flash",
        displayName: "DeepSeek V4 Flash",
        contextWindow: 1048576,
      },
      {
        model: "deepseek-v4-pro",
        displayName: "DeepSeek V4 Pro",
        contextWindow: 1048576,
      },
    ]),
    category: "cn_official",
    icon: "deepseek",
    iconColor: "#1E88E5",
  },
  {
    name: "xAI (Grok)",
    websiteUrl: "https://x.ai/api",
    apiKeyUrl: "https://console.x.ai",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig("xai", "https://api.x.ai/v1", "grok-4.5"),
    endpointCandidates: ["https://api.x.ai/v1"],
    // xAI exposes /v1/responses as a native endpoint, including the Codex
    // store/include/reasoning fields, so no protocol bridge is required.
    apiFormat: "openai_responses",
    modelCatalog: modelCatalog([
      {
        model: "grok-4.5",
        displayName: "Grok 4.5",
        contextWindow: 500000,
        supportsParallelToolCalls: true,
        inputModalities: ["text", "image"],
      },
    ]),
    category: "third_party",
    icon: "xai",
    iconColor: "#000000",
  },
  {
    name: "xAI (Grok) OAuth",
    websiteUrl: "https://x.ai/grok",
    auth: generateThirdPartyAuth(""),
    // These snapshot values keep the generated Codex config self-describing;
    // the adapter pins api.x.ai and the forwarder injects the managed token.
    config: generateThirdPartyConfig("xai", "https://api.x.ai/v1", "grok-4.5"),
    apiFormat: "openai_responses",
    providerType: "xai_oauth",
    requiresOAuth: true,
    modelCatalog: modelCatalog([
      {
        model: "grok-4.5",
        displayName: "Grok 4.5",
        contextWindow: 500000,
        supportsParallelToolCalls: true,
        inputModalities: ["text", "image"],
      },
    ]),
    category: "third_party",
    icon: "xai",
    iconColor: "#000000",
  },
  {
    name: "AiHubMix",
    websiteUrl: "https://aihubmix.com",
    category: "aggregator",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "aihubmix",
      "https://aihubmix.com/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: [
      "https://aihubmix.com/v1",
      "https://api.aihubmix.com/v1",
    ],
    icon: "aihubmix",
    iconColor: "#006FFB",
  },
  {
    name: "CherryIN",
    websiteUrl: "https://open.cherryin.ai",
    apiKeyUrl: "https://open.cherryin.ai/console/token",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "cherryin",
      "https://open.cherryin.net/v1",
      "openai/gpt-5.6-sol",
    ),
    endpointCandidates: ["https://open.cherryin.net/v1"],
    category: "aggregator",
    icon: "cherryin",
  },
  {
    name: "DMXAPI",
    websiteUrl: "https://www.dmxapi.cn",
    category: "aggregator",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "dmxapi",
      "https://www.dmxapi.cn/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://www.dmxapi.cn/v1"],
    isPartner: true, // 合作伙伴
    partnerPromotionKey: "dmxapi", // 促销信息 i18n key
  },
  {
    name: "PackyCode",
    websiteUrl: "https://www.packyapi.ai",
    apiKeyUrl: "https://www.packyapi.ai/register?aff=cc-switch",
    category: "third_party",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "packycode",
      "https://www.packyapi.ai/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: [
      "https://www.packyapi.ai/v1",
      "https://api-slb.packyapi.ai/v1",
      "https://cf.api.fan",
      "https://slb-v1.api.fan",
      "https://www.packyapi.com",
    ],
    isPartner: true, // 合作伙伴
    partnerPromotionKey: "packycode", // 促销信息 i18n key
    icon: "packycode",
  },
  {
    name: "ClaudeCN",
    websiteUrl: "https://claudecn.top",
    apiKeyUrl: "https://claudecn.top/register?aff=ccswitch",
    category: "third_party",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "claudecn",
      "https://claudecn.top/v1",
      "gpt-5.6-sol",
    ),
    isPartner: true,
    partnerPromotionKey: "claudecn",
    icon: "claudecn",
  },
  {
    name: "RunAPI",
    websiteUrl: "https://runapi.host",
    apiKeyUrl: "https://runapi.host",
    category: "aggregator",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "runapi",
      "https://runapi.host/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://runapi.host/v1", "https://runapi.co/v1"],
    isPartner: true,
    partnerPromotionKey: "runapi",
    icon: "runapi",
  },
  {
    name: "RelaxyCode",
    websiteUrl: "https://www.relaxycode.com",
    apiKeyUrl: "https://www.relaxycode.com/register",
    category: "third_party",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "relaxycode",
      "https://www.relaxycode.com/v1",
      "gpt-5.6-sol",
    ),
    icon: "relaxcode",
  },
  {
    name: "Cubence",
    websiteUrl: "https://cubence.com",
    apiKeyUrl: "https://cubence.com/signup?code=CCSWITCH&source=ccs",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "cubence",
      "https://api.cubence.com/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: [
      "https://api.cubence.com/v1",
      "https://api-cf.cubence.com/v1",
      "https://api-dmit.cubence.com/v1",
      "https://api-bwg.cubence.com/v1",
    ],
    category: "third_party",
    isPartner: true, // 合作伙伴
    partnerPromotionKey: "cubence", // 促销信息 i18n key
    icon: "cubence",
    iconColor: "#000000",
  },
  {
    name: "AIGoCode",
    websiteUrl: "https://aigocode.app",
    apiKeyUrl: "https://aigocode.app/invite/CC-SWITCH",
    category: "third_party",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "aigocode",
      "https://api.aigocode.app",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://api.aigocode.app"],
    isPartner: true, // 合作伙伴
    partnerPromotionKey: "aigocode", // 促销信息 i18n key
    icon: "aigocode",
    iconColor: "#5B7FFF",
  },
  {
    name: "RightCode",
    websiteUrl: "https://www.rightapi.ai",
    apiKeyUrl: "https://www.rightapi.ai/register?aff=CCSWITCH",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "rightcode",
      "https://www.rightapi.ai/codex/v1",
      "gpt-5.6-sol",
    ),
    category: "third_party",
    isPartner: true,
    partnerPromotionKey: "rightcode",
    icon: "rc",
    iconColor: "#E96B2C",
  },
  {
    name: "AICodeMirror",
    websiteUrl: "https://www.aicodemirror.ai",
    apiKeyUrl: "https://www.aicodemirror.ai/register?invitecode=9915W3",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "aicodemirror",
      "https://api.aicodemirror.ai/api/codex/backend-api/codex",
      "gpt-5.6-sol",
    ),
    endpointCandidates: [
      "https://api.aicodemirror.ai/api/codex/backend-api/codex",
    ],
    isPartner: true,
    partnerPromotionKey: "aicodemirror",
    icon: "aicodemirror",
    iconColor: "#000000",
  },
  {
    name: "AICoding",
    websiteUrl: "https://aicoding.inc",
    apiKeyUrl: "https://aicoding.inc/i/CCSWITCH",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "aicoding",
      "https://api.aicoding.inc",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://api.aicoding.inc"],
    isPartner: true,
    partnerPromotionKey: "aicoding",
    icon: "aicoding",
    iconColor: "#000000",
  },
  {
    name: "CrazyRouter",
    websiteUrl: "https://www.crazyrouter.com",
    apiKeyUrl: "https://www.crazyrouter.com/register?aff=OZcm&ref=cc-switch",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "crazyrouter",
      "https://cn.crazyrouter.com/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://cn.crazyrouter.com/v1"],
    isPartner: true,
    partnerPromotionKey: "crazyrouter",
    icon: "crazyrouter",
    iconColor: "#000000",
  },
  {
    name: "SSSAiCode",
    websiteUrl: "https://sssaicodeapi.com",
    apiKeyUrl: "https://sssaicodeapi.com/register?ref=DCP0SM",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "sssaicode",
      "https://node-hk.sssaicodeapi.com/api/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: [
      "https://node-hk.sssaicodeapi.com/api/v1",
      "https://node-hk.sssaiapi.com/api/v1",
      "https://node-cf.sssaicodeapi.com/api/v1",
    ],
    category: "third_party",
    isPartner: true, // 合作伙伴
    partnerPromotionKey: "sssaicode", // 促销信息 i18n key
    icon: "sssaicode",
    iconColor: "#000000",
  },
  {
    name: "Compshare",
    nameKey: "providerForm.presets.ucloud",
    websiteUrl: "https://www.compshare.cn",
    apiKeyUrl:
      "https://www.compshare.cn/coding-plan?ytag=GPU_YY_YX_git_cc-switch",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "compshare",
      "https://api.modelverse.cn/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://api.modelverse.cn/v1"],
    category: "aggregator",
    isPartner: true, // 合作伙伴
    partnerPromotionKey: "ucloud", // 促销信息 i18n key
    icon: "ucloud",
    iconColor: "#000000",
  },
  {
    name: "Compshare Coding Plan",
    nameKey: "providerForm.presets.ucloudCoding",
    websiteUrl: "https://www.compshare.cn",
    apiKeyUrl:
      "https://www.compshare.cn/coding-plan?ytag=GPU_YY_YX_git_cc-switch",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "compshare_coding",
      "https://cp.compshare.cn/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://cp.compshare.cn/v1"],
    category: "aggregator",
    isPartner: true,
    partnerPromotionKey: "ucloud",
    icon: "ucloud",
    iconColor: "#000000",
  },
  {
    name: "Micu",
    websiteUrl: "https://www.micuapi.ai",
    apiKeyUrl: "https://www.micuapi.ai/register?aff=aOYQ",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "micu",
      "https://www.micuapi.ai/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://www.micuapi.ai/v1"],
    category: "third_party",
    isPartner: true, // 合作伙伴
    partnerPromotionKey: "micu", // 促销信息 i18n key
    icon: "micu",
    iconColor: "#000000",
  },
  {
    name: "CTok.ai",
    websiteUrl: "https://ctok.ai",
    apiKeyUrl: "https://ctok.ai",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "ctok",
      "https://api.ctok.ai/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://api.ctok.ai/v1"],
    category: "third_party",
    isPartner: true, // 合作伙伴
    partnerPromotionKey: "ctok", // 促销信息 i18n key
    icon: "ctok",
    iconColor: "#000000",
  },
  {
    name: "LionCCAPI",
    websiteUrl: "https://vibecodingapi.ai",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "lionccapi",
      "https://vibecodingapi.ai/v1",
      "gpt-5.6-sol",
    ),
    category: "third_party",
    isPartner: true,
    partnerPromotionKey: "lionccapi",
    icon: "lioncc",
  },
  {
    name: "E-FlowCode",
    websiteUrl: "https://e-flowcode.cc",
    apiKeyUrl: "https://e-flowcode.cc",
    auth: {
      OPENAI_API_KEY: "",
    },
    config: `model_provider = "e-flowcode"
model = "gpt-5.6-sol"
model_reasoning_effort = "high"
disable_response_storage = true
personality = "pragmatic"

[model_providers.e-flowcode]
name = "e-flowcode"
base_url = "https://e-flowcode.cc/v1"
wire_api = "responses"
requires_openai_auth = true
model_context_window = 1000000
model_auto_compact_token_limit = 9000000`,
    category: "third_party",
    endpointCandidates: ["https://e-flowcode.cc/v1"],
    icon: "eflowcode",
    iconColor: "#000000",
  },
  {
    name: "LemonData",
    websiteUrl: "https://lemondata.cc",
    apiKeyUrl: "https://lemondata.cc/r/FFX1ZDUP",
    category: "third_party",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "lemondata",
      "https://api.lemondata.cc/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://api.lemondata.cc/v1"],
    isPartner: true,
    partnerPromotionKey: "lemondata",
    icon: "lemondata",
  },
  {
    name: "PIPELLM",
    websiteUrl: "https://code.pipellm.ai",
    apiKeyUrl: "https://code.pipellm.ai/login?ref=uvw650za",
    auth: {
      OPENAI_API_KEY: "",
    },
    config: `model_provider = "custom"
model = "gpt-5.6-sol"
model_reasoning_effort = "medium"
disable_response_storage = true

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://cc-api.pipellm.ai/v1"`,
    category: "aggregator",
    endpointCandidates: ["https://cc-api.pipellm.ai/v1"],
    icon: "pipellm",
  },
  {
    name: "OpenRouter",
    websiteUrl: "https://openrouter.ai",
    apiKeyUrl: "https://openrouter.ai/keys",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "openrouter",
      "https://openrouter.ai/api/v1",
      "gpt-5.6-sol",
    ),
    category: "aggregator",
    icon: "openrouter",
    iconColor: "#6566F1",
  },
  {
    name: "TheRouter",
    websiteUrl: "https://therouter.ai",
    apiKeyUrl: "https://dashboard.therouter.ai",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "therouter",
      "https://api.therouter.ai/v1",
      "openai/gpt-5.3-codex",
    ),
    endpointCandidates: ["https://api.therouter.ai/v1"],
    category: "aggregator",
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "openai/gpt-5.3-codex",
        displayName: "GPT-5.3 Codex",
      },
    ]),
  },
  {
    name: "A6API",
    websiteUrl: "https://www.a6api.com",
    apiKeyUrl: "https://a6api.com/register?aff=AqNr",
    category: "aggregator",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "a6api",
      "https://api.a6api.com/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: ["https://api.a6api.com/v1"],
    isPartner: true,
    partnerPromotionKey: "a6api",
    icon: "a6api",
  },
  {
    name: "PPIO",
    websiteUrl: "https://ppio.com",
    apiKeyUrl: "https://ppio.com/activity/ccswitch",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "ppio",
      "https://api.ppio.com/openai/v1",
      "deepseek/deepseek-v4-flash-0731",
    ),
    endpointCandidates: ["https://api.ppio.com/openai/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "deepseek/deepseek-v4-flash-0731",
        displayName: "Deepseek V4 Flash 0731",
        contextWindow: 1048576,
        inputModalities: ["text"],
      },
    ]),
    category: "aggregator",
    isPartner: true,
    partnerPromotionKey: "ppio",
    icon: "ppio",
    iconColor: "#2874FF",
  },
  {
    name: "JieKou AI",
    websiteUrl: "https://jiekou.ai/#model-library",
    apiKeyUrl: "https://jiekou.ai/settings/key-management",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "jiekou",
      "https://api.jiekou.ai/openai/v1",
      "claude-fable-5",
    ),
    endpointCandidates: ["https://api.jiekou.ai/openai/v1"],
    apiFormat: "openai_chat",
    modelCatalog: modelCatalog([
      {
        model: "claude-fable-5",
        displayName: "Claude Fable 5",
        contextWindow: 1000000,
        inputModalities: ["text", "image"],
      },
    ]),
    category: "aggregator",
    icon: "jiekou",
    iconColor: "#000000",
  },
  {
    name: "XycAi",
    websiteUrl: "https://xycai.us",
    apiKeyUrl: "https://xycai.us/register?aff=Uhu9",
    category: "aggregator",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "xycai",
      "https://apicdn.xycai.us/v1",
      "gpt-5.6-sol",
    ),
    endpointCandidates: [
      "https://apicdn.xycai.us/v1",
      "https://apicdn.xyc.ai/v1",
    ],
    isPartner: true,
    partnerPromotionKey: "xycai",
    icon: "xycai",
  },
];
