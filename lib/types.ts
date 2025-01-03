/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

interface components {
	schemas: {
		Blueprint: components['schemas']['Contract'] & { [key: string]: any } & {
			type?: 'meta.blueprint';
			layout?: components['schemas']['BlueprintLayout'];
			skeleton?: { [key: string]: any };
		};
		BlueprintLayout: { [key: string]: any };
		Contract: {
			type: string;
		} & { [key: string]: any };
	};
}

export type ContractObject = components['schemas']['Contract'];
export type BlueprintObject = components['schemas']['Blueprint'];
export type BlueprintLayout = components['schemas']['BlueprintLayout'];

export const CONTEXT = 'meta.context';
export const UNIVERSE = 'meta.universe';
export const MATCHER = 'meta.matcher';
export const BLUEPRINT = 'meta.blueprint';
