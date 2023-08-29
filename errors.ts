interface Event extends Error {
  name: "Interrupt" | "Feedback" | "Exit" | "Internal";
  inner_error?: Error;
}

interface InterruptError extends Event {
  name: "Interrupt";
  inner_error: Error;
}

type FeedbackMessage = {
  role: "user" | "system";
  prompt: string;
}

interface FeedbackError extends Event {
  name: "Feedback";
  feedback_message: FeedbackMessage;
}

interface ExitError<O> extends Event {
  name: "Exit";
  output?: O;
}

interface InternalError extends Event {
  name: "Internal";
  exit_code?: number;
}

type RuntimeErrorParams = {
  code?: number;
  context?: Error;
};

export class Internal extends Error implements InternalError {
  name: "Internal" = "Internal";
  inner_error?: Error;
  exit_code?: number;

  constructor(message: string, { code, context }: RuntimeErrorParams = {}) {
    super(message);
    this.exit_code = code;
    this.inner_error = context;
  }
}

export class Interrupt extends Error implements InterruptError {
  name: "Interrupt" = "Interrupt";
  inner_error: Error;

  constructor(value: Error) {
    super("LLM execution interrupted");
    this.inner_error = value;
  }
}

export class Feedback extends Error implements FeedbackError {
  name: "Feedback" = "Feedback";
  feedback_message: FeedbackMessage;

  constructor(message: string) {
    super(message);
    this.feedback_message = {
      role: "system",
      prompt: message,
    }
  }
}

export class Exit<O> extends Error implements ExitError<O> {
  name: "Exit" = "Exit";
  output?: O;

  constructor(value?: O) {
    super("LLM execution exited");
    this.output = value;
  }
}
