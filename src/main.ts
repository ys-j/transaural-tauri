import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const form = document.forms.namedItem("audio");
const switchButton = form?.["switch"] as HTMLButtonElement | undefined;
const inputSelect = form?.["input_device"] as HTMLSelectElement | undefined;
const outputSelect = form?.["output_device"] as HTMLSelectElement | undefined;
const latencyInput = form?.["master_latency"] as HTMLInputElement | undefined;
const listenerCoordXInput = form?.["listener_coord_x"] as HTMLInputElement | undefined;
const listenerCoordYInput = form?.["listener_coord_y"] as HTMLInputElement | undefined;
const leftSpeakerCoordXInput = form?.["speaker_coord_lx"] as HTMLInputElement | undefined;
const leftSpeakerCoordYInput = form?.["speaker_coord_ly"] as HTMLInputElement | undefined;
const rightSpeakerCoordXInput = form?.["speaker_coord_rx"] as HTMLInputElement | undefined;
const rightSpeakerCoordYInput = form?.["speaker_coord_ry"] as HTMLInputElement | undefined;
const interauralDistanceInput = form?.["interaural_distance"] as HTMLInputElement | undefined;
const masterGainInput = form?.["master_gain"] as HTMLInputElement | undefined;
const attenuationInput = form?.["canceling_attenuation"] as HTMLInputElement | undefined;
const lowpassCutoffMinInput = form?.["lowpass_cutoff_min"] as HTMLInputElement | undefined;
const highpassCutoffInput = form?.["highpass_cutoff"] as HTMLInputElement | undefined;
const lowshelfCutoffInput = form?.["lowshelf_cutoff"] as HTMLInputElement | undefined;
const lowshelfGainInput = form?.["lowshelf_gain"] as HTMLInputElement | undefined;
const wetDryInput = form?.["wet_dry"] as HTMLInputElement | undefined;
const temperatureInput = form?.["temperature"] as HTMLInputElement | undefined;

const state = new Proxy({
	turned: false,
}, {
	set(target, prop, val, _receiver) {
		switch (prop) {
			case "turned":
				if (switchButton) {
					switchButton.dataset.turn = val ? "on" : "off";
				}
				for (const fs of form?.getElementsByTagName("fieldset") || []) {
					fs.disabled = val;
				}
				break;
			default:
				throw new Error("No property in state object.");
		}
		target[prop] = val;
		return true;
	},
});

async function fetchAudioDevices() {
	const devices: AudioDeviceDescription[] = await invoke("get_audio_devices");
	for (const d of devices) {
		const opt = new Option(d.name, d.id, d.isDefault, d.isDefault);
		if (d.driver) opt.dataset.driver = d.driver;
		switch (d.direction) {
			case "input":
				inputSelect?.options.add(opt);
				break;
			case "output":
				outputSelect?.options.add(opt);
				break;
		}
	}
}

function updatePositionFigure() {
	// @ts-ignore
	const svg = document.getElementById("svg-positions") as SVGSVGElement;

	const vListenerX = listenerCoordXInput?.value;
	const vListenerY = listenerCoordYInput?.value;
	const vSpeakerLX = leftSpeakerCoordXInput?.value;
	const vSpeakerLY = leftSpeakerCoordYInput?.value;
	const vSpeakerRX = rightSpeakerCoordXInput?.value;
	const vSpeakerRY = rightSpeakerCoordYInput?.value;
	
	if (vListenerX && vListenerY) {
		const pListener = svg.getElementById("listener");
		pListener?.setAttribute("cx", vListenerX);
		pListener?.setAttribute("cy", `${-vListenerY}`);
	}
	if (vSpeakerLX && vSpeakerLY) {
		const pSpeakerL = svg.getElementById("speaker_l");
		pSpeakerL?.setAttribute("cx", vSpeakerLX);
		pSpeakerL?.setAttribute("cy", `${-vSpeakerLY}`);
	}
	if (vSpeakerRX && vSpeakerRY) {
		const pSpeakerR = svg.getElementById("speaker_r");
		pSpeakerR?.setAttribute("cx", vSpeakerRX);
		pSpeakerR?.setAttribute("cy", `${-vSpeakerRY}`);
	}
}

function saveConfig() {
	if (!inputSelect || !outputSelect) return;
	const halfInterauralDistance = (interauralDistanceInput?.valueAsNumber ?? 16) / 200;
	const listenerCoordX = (listenerCoordXInput?.valueAsNumber ?? 0) / 100;
	const listenerCoordY = (listenerCoordYInput?.valueAsNumber ?? 0) / 100;
	const config: InvokeOptions = {
		inputId: inputSelect.value,
		outputId: outputSelect.value,
		latency: latencyInput?.valueAsNumber ?? 100,
		position: {
			leftSpeaker: [
				(leftSpeakerCoordXInput?.valueAsNumber ?? 60) / 100,
				(leftSpeakerCoordYInput?.valueAsNumber ?? 60) / 100,
			],
			rightSpeaker: [
				(rightSpeakerCoordXInput?.valueAsNumber ?? 60) / 100,
				(rightSpeakerCoordYInput?.valueAsNumber ?? 60) / 100,
			],
			leftEar: [ listenerCoordX - halfInterauralDistance, listenerCoordY ],
			rightEar: [ listenerCoordX + halfInterauralDistance, listenerCoordY ],
		},
		masterGain: (masterGainInput?.valueAsNumber ?? 75) / 100,
		attenuation: (attenuationInput?.valueAsNumber ?? 70) / 100,
		lowpassCutoffMin: lowpassCutoffMinInput?.valueAsNumber ?? 800,
		highpassCutoff: highpassCutoffInput?.valueAsNumber ?? 50,
		lowshelfCutoff: lowshelfCutoffInput?.valueAsNumber ?? 200,
		lowshelfGain: lowshelfGainInput?.valueAsNumber ?? 3,
		wetDry: (wetDryInput?.valueAsNumber ?? 100) / 100,
		temperature: temperatureInput?.valueAsNumber ?? 20,
	};
	localStorage.setItem("config", JSON.stringify(config));
	return config;
}

function restoreConfig() {
	const config = localStorage.getItem("config");
	if (!config) return;
	const options: InvokeOptions = JSON.parse(config);
	if (inputSelect) inputSelect.value = options.inputId;
	if (outputSelect) outputSelect.value = options.outputId;
	if (latencyInput) latencyInput.valueAsNumber = options.latency;
	if (leftSpeakerCoordXInput && leftSpeakerCoordYInput) {
		[
			leftSpeakerCoordXInput.valueAsNumber,
			leftSpeakerCoordYInput.valueAsNumber,
		] = options.position.leftSpeaker.map(v => (v * 100) | 0);
	}
	if (rightSpeakerCoordXInput && rightSpeakerCoordYInput) {
		[
			rightSpeakerCoordXInput.valueAsNumber,
			rightSpeakerCoordYInput.valueAsNumber,
		] = options.position.rightSpeaker.map(v => (v * 100) | 0);
	}
	if (interauralDistanceInput) {
		const abs = Math.abs(options.position.leftEar[0] - options.position.rightEar[0]);
		interauralDistanceInput.valueAsNumber = (abs * 100) | 0;
	}
	if (masterGainInput) masterGainInput.valueAsNumber = (options.masterGain * 100) | 0;
	if (attenuationInput) attenuationInput.valueAsNumber = (options.attenuation * 100) | 0;
	if (highpassCutoffInput) highpassCutoffInput.valueAsNumber = options.highpassCutoff;
	if (lowshelfCutoffInput) lowshelfCutoffInput.valueAsNumber = options.lowshelfCutoff;
	if (lowshelfGainInput) lowshelfGainInput.valueAsNumber = options.lowshelfGain;
	if (wetDryInput) wetDryInput.valueAsNumber = (options.wetDry * 100) | 0;
	if (temperatureInput) temperatureInput.valueAsNumber = options.temperature;
}

// init
fetchAudioDevices();
restoreConfig();
updatePositionFigure();

(form?.["positions"] as HTMLFieldSetElement).addEventListener("change", updatePositionFigure);

form?.addEventListener("submit", e => {
	e.preventDefault();
	if (state.turned) {
		invoke("abort_audio_routing");
	} else {
		const config = saveConfig();
		if (!config) return;
		listen<Payload>("finished", e => {
			state.turned = !e.payload.isFinished;
		}).then(() => {
			invoke("set_audio_devices", config);
		}).then(() => {
			state.turned = true;
		});
	}
});

window.addEventListener("unload", () => {
	invoke("abort_audio_routing");
	saveConfig();
});