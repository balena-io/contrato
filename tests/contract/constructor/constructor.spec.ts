/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../../chai';

import Contract from '../../../lib/contract';

describe('Contract constructor', () => {
	it('should create a simple contract', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		expect(contract.raw()).to.deep.equal({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
		});

		expect(contract.getType()).to.equal('arch.sw');
		expect(contract.getSlug()).to.equal('armv7hf');
		expect(contract.getVersion()).to.equal(undefined);
		expect(contract.getCanonicalSlug()).to.equal('armv7hf');
		expect(contract.hash()).to.equal(
			'471d73db7a92c2e05b5b1426805f7bdd85659741f1366f1c06955a3d39b3ea68',
		);
	});

	it('should should allow extra fields on round trip', () => {
		const contract = new Contract({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			xxx: '123',
		});

		expect(contract.raw()).to.deep.equal({
			type: 'arch.sw',
			name: 'armv7hf',
			slug: 'armv7hf',
			xxx: '123',
		});

		expect(contract.getType()).to.equal('arch.sw');
		expect(contract.getSlug()).to.equal('armv7hf');
		expect(contract.getVersion()).to.equal(undefined);
		expect(contract.getCanonicalSlug()).to.equal('armv7hf');
		expect(contract.hash()).to.equal(
			'8d09f2bcabd5eec6ff6d619c07a4fdd8de2ab184b5aaab647fdb85594420b758',
		);
	});
});
