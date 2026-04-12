import { expect } from '../chai';

import Contract from '../../lib/contract';

describe('Contract interpolate', () => {
	it('should resolve templates at construction time', () => {
		const contract = new Contract({
			name: 'Debian {{this.data.codename}}',
			slug: 'debian',
			version: 'wheezy',
			type: 'sw.os',
			data: {
				codename: 'Wheezy',
				url: 'https://contracts.org/downloads/{{this.type}}/{{this.slug}}/{{this.version}}.tar.gz',
			},
		});

		expect(contract.raw).to.deep.equal({
			name: 'Debian Wheezy',
			slug: 'debian',
			version: 'wheezy',
			type: 'sw.os',
			data: {
				codename: 'Wheezy',
				url: 'https://contracts.org/downloads/sw.os/debian/wheezy.tar.gz',
			},
		});
	});

	it('should not resolve templates for which values do not exist', () => {
		const contract = new Contract({
			name: 'Debian {{this.data.codename}}',
			slug: 'debian',
			version: 'wheezy',
			type: 'sw.os',
			data: {
				url: 'https://contracts.org/downloads/{{this.type}}/{{this.slug}}/{{this.version}}.tar.gz',
			},
		});

		expect(contract.raw.name).to.equal('Debian {{this.data.codename}}');
		expect(contract.raw.data.url).to.equal(
			'https://contracts.org/downloads/sw.os/debian/wheezy.tar.gz',
		);
	});

	it('should return the contract instance', () => {
		const contract = new Contract({
			name: 'Debian {{this.data.codename}}',
			slug: 'debian',
			version: 'wheezy',
			type: 'sw.os',
			data: {
				url: 'https://contracts.org/downloads/{{this.type}}/{{this.slug}}/{{this.version}}.tar.gz',
			},
		});

		expect(contract.interpolate()).to.equal(contract);
	});

	it('should not perform interpolation on children', () => {
		const contract = new Contract({
			slug: 'debian',
			version: 'wheezy',
			type: 'sw.os',
			children: {
				foo: {
					bar: {
						slug: '{{this.version}}-child',
						type: 'foo.bar',
					},
				},
			},
		});

		const children = contract.getChildren();
		expect(children).to.have.lengthOf(1);
		expect(children[0].getSlug()).to.equal('{{this.version}}-child');
	});
});
