/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../../chai';

import Contract from '../../../lib/contract';

describe('Contract templates', () => {
	it('should resolve templates for which the values exist', () => {
		const contract = new Contract({
			type: 'arch.sw',
			version: '7',
			name: 'ARM v{{this.version}}',
			slug: 'armv7hf',
		});

		expect(contract.raw).to.deep.equal({
			type: 'arch.sw',
			version: '7',
			name: 'ARM v7',
			slug: 'armv7hf',
		});
	});

	it('should not resolve templates for which the values do not exist', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: '{{this.displayName}}',
			slug: 'armv7hf',
		});

		expect(contract.raw).to.deep.equal({
			type: 'arch.sw',
			name: '{{this.displayName}}',
			slug: 'armv7hf',
		});
	});
});
