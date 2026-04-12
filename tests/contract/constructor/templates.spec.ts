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

		expect(contract.hash()).to.equal(
			'5673c1975905fd6ccbc6d605b36d21121c5f04e5ffac23480dde3cef1df77878',
		);

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

		expect(contract.hash()).to.equal(
			'1240b9d276e7da5e633f3b280ce2bd94457ad5ca897146fc38bd1db831ba86cf',
		);

		expect(contract.raw).to.deep.equal({
			type: 'arch.sw',
			name: '{{this.displayName}}',
			slug: 'armv7hf',
		});
	});
});
