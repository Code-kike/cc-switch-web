import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps, PropsWithChildren } from "react";
import { useForm } from "react-hook-form";
import { describe, expect, it, vi } from "vitest";
import { HermesFormFields } from "@/components/providers/forms/HermesFormFields";
import { Form } from "@/components/ui/form";

type HermesFormFieldsProps = ComponentProps<typeof HermesFormFields>;

const FormShell = ({ children }: PropsWithChildren) => {
  const form = useForm();

  return <Form {...form}>{children}</Form>;
};

const renderHermesForm = (overrides: Partial<HermesFormFieldsProps> = {}) => {
  const props: HermesFormFieldsProps = {
    baseUrl: "https://api.example.com/v1",
    onBaseUrlChange: vi.fn(),
    apiKey: "sk-test",
    onApiKeyChange: vi.fn(),
    category: "custom",
    shouldShowApiKeyLink: false,
    websiteUrl: "",
    apiMode: "chat_completions",
    onApiModeChange: vi.fn(),
    models: [
      { id: "model-a", name: "Model A" },
      { id: "model-b", name: "Model B" },
    ],
    onModelsChange: vi.fn(),
    rateLimitDelay: 0.5,
    onRateLimitDelayChange: vi.fn(),
    ...overrides,
  };

  return {
    props,
    ...render(
      <FormShell>
        <HermesFormFields {...props} />
      </FormShell>,
    ),
  };
};

describe("HermesFormFields", () => {
  // d9d4a660: IME composition must stay local until the platform commits,
  // otherwise a parent re-render overwrites the browser-managed marked text.
  it("keeps model name composition local until the IME commits", () => {
    const onModelsChange = vi.fn();
    const { props, rerender } = renderHermesForm({ onModelsChange });
    const modelNameInput = screen.getByDisplayValue("Model A");

    fireEvent.compositionStart(modelNameInput);
    fireEvent.change(modelNameInput, {
      target: { value: "mimomimo" },
    });

    expect(modelNameInput).toHaveValue("mimomimo");
    expect(onModelsChange).not.toHaveBeenCalled();

    rerender(
      <FormShell>
        <HermesFormFields {...props} />
      </FormShell>,
    );
    expect(modelNameInput).toHaveValue("mimomimo");

    fireEvent.compositionEnd(modelNameInput, {
      data: "mimomimo",
      target: { value: "mimomimo" },
    });

    expect(onModelsChange).toHaveBeenCalledTimes(1);
    expect(onModelsChange).toHaveBeenCalledWith([
      { ...props.models[0], name: "mimomimo" },
      props.models[1],
    ]);
  });

  it("keeps base url composition local until the IME commits", () => {
    const onBaseUrlChange = vi.fn();
    const { props, rerender } = renderHermesForm({ onBaseUrlChange });
    const baseUrlInput = document.getElementById("hermes-baseurl");
    expect(baseUrlInput).not.toBeNull();

    fireEvent.compositionStart(baseUrlInput!);
    fireEvent.change(baseUrlInput!, {
      target: { value: "https://api.example.com/v1中文" },
    });

    expect(baseUrlInput).toHaveValue("https://api.example.com/v1中文");
    expect(onBaseUrlChange).not.toHaveBeenCalled();

    rerender(
      <FormShell>
        <HermesFormFields {...props} />
      </FormShell>,
    );
    expect(baseUrlInput).toHaveValue("https://api.example.com/v1中文");

    fireEvent.compositionEnd(baseUrlInput!, {
      data: "文",
      target: { value: "https://api.example.com/v1中文" },
    });

    expect(onBaseUrlChange).toHaveBeenCalledTimes(1);
    expect(onBaseUrlChange).toHaveBeenCalledWith("https://api.example.com/v1中文");
  });
});
