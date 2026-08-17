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
			'471d73db7a92c2e05b5b1426805f7bdd85659741f1366f1c06955a3d39b3ea68',
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
