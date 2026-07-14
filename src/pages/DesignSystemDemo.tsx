import { useState } from "react";
import { Button, Input, Modal, Select, Spinner, Toggle } from "@/components/ui";

export function DesignSystemDemo() {
  const [modalOpen, setModalOpen] = useState(false);
  const [toggleOn, setToggleOn] = useState(true);
  const [selectValue, setSelectValue] = useState("balanced");

  return (
    <div className="mx-auto max-w-4xl space-y-10 px-6 py-10">
      <header className="space-y-2 border-b border-border pb-6">
        <p className="font-mono text-xs uppercase tracking-widest text-accent">Sprint 1</p>
        <h1 className="text-4xl text-text-primary">Design System</h1>
        <p className="text-text-secondary">
          Trustworthy, professional, and calm — editorial typography with deep green accent.
        </p>
      </header>

      <section className="space-y-4">
        <h2 className="text-2xl text-accent">Buttons</h2>
        <div className="flex flex-wrap gap-3">
          <Button>Primary</Button>
          <Button variant="secondary">Secondary</Button>
          <Button variant="ghost">Ghost</Button>
          <Button variant="destructive">Destructive</Button>
          <Button loading>Loading</Button>
          <Button disabled>Disabled</Button>
        </div>
        <div className="flex flex-wrap gap-3">
          <Button size="sm">Small</Button>
          <Button size="md">Medium</Button>
          <Button size="lg">Large</Button>
        </div>
      </section>

      <section className="space-y-4">
        <h2 className="text-2xl text-accent">Inputs</h2>
        <div className="grid gap-4 md:grid-cols-2">
          <Input
            label="Session name"
            placeholder="Q3 Analysis"
            helperText="Visible in the sidebar"
          />
          <Input label="API key" type="password" error="Invalid key format" />
        </div>
      </section>

      <section className="space-y-4">
        <h2 className="text-2xl text-accent">Select</h2>
        <Select
          label="Privacy level"
          value={selectValue}
          onChange={(e) => setSelectValue(e.target.value)}
          options={[
            { value: "strict", label: "Strict" },
            { value: "balanced", label: "Balanced" },
            { value: "permissive", label: "Permissive" },
          ]}
        />
      </section>

      <section className="space-y-4">
        <h2 className="text-2xl text-accent">Toggle &amp; Spinner</h2>
        <div className="flex items-center gap-6">
          <Toggle checked={toggleOn} onChange={setToggleOn} label="Research mode" />
          <Spinner />
        </div>
      </section>

      <section className="space-y-4">
        <h2 className="text-2xl text-accent">Modal</h2>
        <Button onClick={() => setModalOpen(true)}>Open modal</Button>
        <Modal
          open={modalOpen}
          onClose={() => setModalOpen(false)}
          title="Research plan preview"
          footer={
            <>
              <Button variant="ghost" onClick={() => setModalOpen(false)}>
                Cancel
              </Button>
              <Button onClick={() => setModalOpen(false)}>Approve</Button>
            </>
          }
        >
          <p className="text-text-secondary">
            Modal with focus trap, overlay click, and Escape to close.
          </p>
        </Modal>
      </section>
    </div>
  );
}
