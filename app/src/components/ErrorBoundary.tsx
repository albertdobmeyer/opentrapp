import { Component, type ReactNode } from "react";


import ContactSupport from "./failure/ContactSupport";
import FriendlyRetry from "./failure/FriendlyRetry";

import { classifyError } from "@/lib/errors";

interface Props {
  children: ReactNode;
  /** Optional title override surfaced through to the failure component. */
  fallbackTitle?: string;
  /** When true, skip Level 2 and go straight to ContactSupport. Defaults to false. */
  forceContactSupport?: boolean;
}

interface State {
  hasError: boolean;
  error: Error | null;
  /** Set to true when the user clicks "Get help" from FriendlyRetry. */
  escalated: boolean;
}

/**
 * Spec 06 cascade:
 *   Level 1 (silent retry) — handled at the operation level by hooks/wrappers.
 *   Level 2 FriendlyRetry — shown for retryable / transient classifications.
 *   Level 3 ContactSupport — shown for unrecoverable classifications, after Level 2 escalation,
 *                            or always when forceContactSupport is set.
 */
export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null, escalated: false };
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: { componentStack?: string | null }) {
    // Surface to the dev console; never to the end user.
    console.error("ErrorBoundary caught:", error, info.componentStack);
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null, escalated: false });
  };

  handleEscalate = () => {
    this.setState({ escalated: true });
  };

  render() {
    if (!this.state.hasError || !this.state.error) {
      return this.props.children;
    }

    const classified = classifyError(this.state.error);
    const useLevel3 =
      (this.props.forceContactSupport ?? false) ||
      this.state.escalated ||
      !classified.retryable;

    if (useLevel3) {
      return (
        <ContactSupport
          classified={classified}
          onRetry={classified.retryable ? this.handleRetry : undefined}
          titleOverride={this.props.fallbackTitle}
        />
      );
    }

    return (
      <FriendlyRetry
        classified={classified}
        onRetry={this.handleRetry}
        onGetHelp={this.handleEscalate}
        titleOverride={this.props.fallbackTitle}
      />
    );
  }
}
