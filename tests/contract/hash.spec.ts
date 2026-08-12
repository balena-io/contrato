/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';

describe('Contract hash', () => {
	it('should compute the hash from the contract raw object', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		expect(contract.hash()).to.equal(
			'e3d3b7f2e5820db4b45975380a3f467bc2ff2999',
		);
	});

	it('should produce different hashes for different contracts', () => {
		const contract1 = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});
		const contract2 = new Contract({
			type: 'arch.sw',
			name: 'aarch64',
			slug: 'aarch64',
		});

		expect(contract1.hash()).to.not.equal(contract2.hash());
	});
});
