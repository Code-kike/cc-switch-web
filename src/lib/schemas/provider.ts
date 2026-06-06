import { z } from "zod";

/**
 * 解析 JSON 语法错误，提取位置信息
 */
function parseJsonError(error: unknown): string {
  if (!(error instanceof SyntaxError)) {
    return "配置 JSON 格式错误";
  }

  const message = error.message;

  // 提取位置信息：Chrome/V8: "Unexpected token ... in JSON at position 123"
  const positionMatch = message.match(/at position (\d+)/i);
  if (positionMatch) {
    const position = parseInt(positionMatch[1], 10);
    return `JSON 格式错误：${message.split(" in JSON")[0]}（位置：${position}）`;
  }

  // Firefox: "JSON.parse: unexpected character at line 1 column 23"
  const lineColumnMatch = message.match(/line (\d+) column (\d+)/i);
  if (lineColumnMatch) {
    const line = lineColumnMatch[1];
    const column = lineColumnMatch[2];
    return `JSON 格式错误：第 ${line} 行，第 ${column} 列`;
  }

  // 通用情况：提取关键错误信息
  const cleanMessage = message
    .replace(/^JSON\.parse:\s*/i, "")
    .replace(/^Unexpected\s+/i, "意外的 ")
    .replace(/token/gi, "符号")
    .replace(/Expected/gi, "预期");

  return `JSON 格式错误：${cleanMessage}`;
}

export const providerSchema = z.object({
  // 规范 schema：供应商名称必填（去除首尾空白后非空）。
  // 注意：react-hook-form 解析器使用下方的 providerFormSchema（name 放宽为软校验），
  // 空/纯空白名称不在解析器层硬拒绝，而是交由 handleSubmit 的确认框处理，
  // 用户可在确认后保存暂时为空的名称（见 ProviderFormSoftValidation 测试）。
  name: z
    .string()
    .refine((value) => value.trim().length > 0, "请填写供应商名称"),
  websiteUrl: z.string().url("请输入有效的网址").optional().or(z.literal("")),
  notes: z.string().optional(),
  settingsConfig: z
    .string()
    .min(1, "请填写配置内容")
    .superRefine((value, ctx) => {
      let parsed: unknown;
      try {
        parsed = JSON.parse(value);
      } catch (error) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: parseJsonError(error),
        });
        return;
      }
      // 顶层配置必须是 JSON 对象（与 codex/gemini hook 内部校验一致），
      // 避免把合法 JSON 但非对象（数组 / 字面量）误存为有效配置。
      if (
        typeof parsed !== "object" ||
        parsed === null ||
        Array.isArray(parsed)
      ) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: "配置必须是 JSON 对象",
        });
      }
    }),
  // 图标配置
  icon: z.string().optional(),
  iconColor: z.string().optional(),
});

/**
 * react-hook-form 解析器专用的 schema：
 * 将 name 放宽为软校验（不在解析器层硬拒绝空/纯空白名称），
 * 空名称由 handleSubmit 的确认框（软校验）处理，保持既有交互不变。
 * 其余规则（settingsConfig 等）与 providerSchema 完全一致。
 */
export const providerFormSchema = providerSchema.extend({
  name: z.string(),
});

export type ProviderFormData = z.infer<typeof providerSchema>;
