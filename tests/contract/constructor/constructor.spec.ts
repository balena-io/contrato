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
			'e3d3b7f2e5820db4b45975380a3f467bc2ff2999',
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
			'5fd152eb6917143accc7fea3771e92835ffe0c50',
		);
	});
});
