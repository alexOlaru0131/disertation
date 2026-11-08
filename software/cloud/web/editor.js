import { anglesForFoot, legJoints, norm, transform } from "./kinematics.js";

const byId = (id) => document.getElementById(id);
const rounded = (value) => Number(value.toFixed(5)).toString();

export class PoseEditor {
  constructor(onChange) {
    this.pose = null;
    this.onChange = onChange;
    this.component = byId("component");
    this.plane = byId("drag-plane");
    this.controls = byId("editor-controls");
    this.message = byId("editor-message");
    this.exportButton = byId("export-pose");
    this.rows = [];
    this.inputs = {};

    for (const [kind, label] of [["foot", "Vârfuri"], ["mount", "Prinderi — motorul 1"]]) {
      const group = document.createElement("optgroup");
      group.label = label;
      for (let i = 0; i < 6; i++) group.append(new Option(`Picior ${i + 1}`, `${kind}:${i}`));
      this.component.append(group);
    }
    for (const kind of ["position", "rotation"]) {
      this.inputs[kind] = ["X", "Y", "Z"].map((axis) => {
        const label = document.createElement("label");
        const input = document.createElement("input");
        input.type = "number";
        input.step = kind === "position" ? "0.01" : "1";
        input.required = true;
        input.dataset.kind = kind;
        input.setAttribute("aria-label", `${kind === "position" ? "Poziție" : "Rotație"} ${axis}`);
        label.append(axis, input);
        byId(`${kind}-fields`).append(label);
        return input;
      });
    }
    for (let index = 0; index < 6; index++) {
      const row = document.createElement("tr");
      const cell = row.insertCell();
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = String(index + 1);
      button.setAttribute("aria-label", `Selectează vârful piciorului ${index + 1}`);
      button.addEventListener("click", () => this.select(`foot:${index}`));
      cell.append(button);
      const coordinates = Array.from({ length: 3 }, () => row.insertCell());
      coordinates.forEach((cell) => { cell.textContent = "—"; });
      byId("feet-positions").append(row);
      this.rows.push({ row, coordinates });
    }

    this.component.addEventListener("change", () => this.refresh());
    this.plane.addEventListener("change", () => this.onChange());
    this.controls.addEventListener("input", (event) => {
      const kind = event.target.dataset.kind;
      if (!kind) return;
      const values = this.inputs[kind].map((input) => input.valueAsNumber);
      if (values.every(Number.isFinite)) this.update(kind, values, true);
    });
    this.controls.addEventListener("change", (event) => {
      if (event.target.dataset.kind) this.refresh();
    });
    this.exportButton.addEventListener("click", () => this.export());
  }

  get selection() { return this.component.value; }

  selected() {
    const [kind, index] = this.selection.split(":");
    return { kind, index: Number(index), leg: this.pose?.legs[Number(index)] };
  }

  select(key) {
    if (!this.pose) return;
    this.component.value = key;
    this.refresh();
  }

  load(state) {
    this.pose = structuredClone(state);
    const unresolved = [];
    this.pose.legs.forEach((leg, index) => {
      try { leg.angles = anglesForFoot(leg, leg.foot); }
      catch { unresolved.push(index + 1); }
    });
    this.controls.disabled = false;
    this.exportButton.disabled = false;
    this.notify(unresolved.length
      ? `Vârfuri inițiale în afara razei: ${unresolved.join(", ")}. Selectează-le pentru ajustare.`
      : "Trage componenta selectată sau introdu coordonatele. Lungimile sunt în metri.", unresolved.length > 0);
    this.refresh();
  }

  notify(text, error = false) {
    this.message.textContent = text;
    this.message.dataset.error = String(error);
  }

  point() {
    if (!this.pose) return null;
    const { kind, leg } = this.selected();
    if (kind === "body") return this.pose.body.position;
    return transform(this.pose.body.position, this.pose.body.rotation).toWorld(kind === "foot" ? leg.foot : leg.motor);
  }

  move(world) {
    if (!this.pose) return false;
    const local = this.selection === "body" ? world
      : transform(this.pose.body.position, this.pose.body.rotation).toLocal(world);
    return this.update("position", local);
  }

  update(field, values, keepInput = false) {
    if (!this.pose || !values.every(Number.isFinite)) return false;
    const { kind, leg } = this.selected();
    try {
      if (kind === "body") {
        this.pose.body[field] = values;
      } else if (kind === "foot") {
        if (field !== "position") return false;
        const angles = anglesForFoot(leg, values);
        leg.foot = values;
        leg.angles = angles;
      } else {
        leg[field === "position" ? "motor" : "motorRotation"] = values;
        leg.centerToMotor = norm(leg.motor);
        leg.foot = legJoints(leg)[3];
      }
      this.notify("Postură actualizată · vârfurile sunt afișate în coordonate locale corpului.");
      this.refresh(keepInput);
      return true;
    } catch (error) {
      this.notify(error.message, true);
      return false;
    }
  }

  refresh(keepInput = false) {
    if (!this.pose) return;
    const { kind, index, leg } = this.selected();
    const position = kind === "body" ? this.pose.body.position : kind === "foot" ? leg.foot : leg.motor;
    const rotation = kind === "body" ? this.pose.body.rotation : leg.motorRotation;
    byId("position-label").textContent = kind === "body" ? "Poziție globală (m)" : "Poziție locală corpului (m)";
    byId("rotation-group").hidden = kind === "foot";
    for (const [field, values] of [["position", position], ["rotation", rotation]]) {
      this.inputs[field].forEach((input, i) => {
        if (!keepInput || input !== document.activeElement) input.value = rounded(values[i]);
      });
    }
    this.rows.forEach(({ row, coordinates }, i) => {
      row.classList.toggle("selected", kind !== "body" && index === i);
      coordinates.forEach((cell, axis) => { cell.textContent = this.pose.legs[i].foot[axis].toFixed(4); });
    });
    this.onChange();
  }

  export() {
    if (!this.pose) return;
    const data = {
      units: "m",
      frame: "body",
      body: { position: this.pose.body.position, rotation_degrees: this.pose.body.rotation },
      feet: this.pose.legs.map(({ foot }, i) => ({ leg: i + 1, x: foot[0], y: foot[1], z: foot[2] })),
    };
    const url = URL.createObjectURL(new Blob([JSON.stringify(data, null, 2)], { type: "application/json" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = "hexapod-postura.json";
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
}
