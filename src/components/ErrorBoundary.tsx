import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";
import i18n from "@/i18n";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
}

/**
 * 顶层错误边界：捕获渲染期抛出的异常，避免整个应用白屏（item 14）。
 *
 * 文案用 `i18n.t` + `defaultValue` 取——错误边界是类组件、运行在 React hooks
 * 之外，无法用 `useTranslation`，故直接复用已初始化的 i18n 实例；即便对应 key
 * 缺失，defaultValue 也能保证 fallback 始终有可读内容。
 */
export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error(
      "[ErrorBoundary] Uncaught render error:",
      error,
      info.componentStack,
    );
  }

  private handleReload = (): void => {
    if (typeof window !== "undefined") {
      window.location.reload();
    }
  };

  render(): ReactNode {
    if (!this.state.hasError) {
      return this.props.children;
    }

    return (
      <div
        role="alert"
        className="flex h-screen w-screen flex-col items-center justify-center gap-4 p-8 text-center"
      >
        <h1 className="text-lg font-semibold">
          {i18n.t("errors.renderCrashTitle", { defaultValue: "出错了" })}
        </h1>
        <p className="max-w-md text-sm text-muted-foreground">
          {i18n.t("errors.renderCrashMessage", {
            defaultValue: "界面遇到意外错误，无法继续。重新加载通常即可恢复。",
          })}
        </p>
        <button
          type="button"
          onClick={this.handleReload}
          className="rounded-md border border-input bg-background px-4 py-2 text-sm font-medium shadow-sm transition-colors hover:bg-accent hover:text-accent-foreground"
        >
          {i18n.t("errors.renderCrashReload", { defaultValue: "重新加载" })}
        </button>
      </div>
    );
  }
}

export default ErrorBoundary;
