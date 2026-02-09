type AudioDeviceDescription = {
	id: string,
	name: string,
	driver?: string,
	direction: string,
	isDefault: boolean,
}

type PositionCoords = {
	leftSpeaker: [number, number],
	rightSpeaker: [number, number],
	leftEar: [number, number],
	rightEar: [number, number],
}

type InvokeOptions = {
	inputId: string,
	outputId: string,
	latency: number,
	position: PositionCoords,
	masterGain: number,
	attenuation: number,
	lowpassCutoffMin: number,
	highpassCutoff: number,
	lowshelfCutoff: number,
	lowshelfGain: number,
	wetDry: number,
	temperature: number,
}

type Payload = {
	isFinished: boolean,
}