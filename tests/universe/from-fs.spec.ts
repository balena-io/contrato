import path from 'path';
import { expect } from '../chai';

import Contract from '../../lib/contract';
import Universe from '../../lib/universe';

describe('Universe fromFs', () => {
	it('allows loading a universe from a directory', async () => {
		const universe = await Universe.fromFs(path.join(__dirname, './contracts'));

		expect(
			universe.findChildren(
				Contract.createMatcher({ type: 'hw.device-type', slug: 'raspberrypi' }),
			),
		).to.have.lengthOf(1);
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				}),
			),
		).to.have.lengthOf(1);
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'intel-nuc',
				}),
			),
		).to.have.lengthOf(1);
	});

	it('allows loading only canonical contracts from a directory', async () => {
		const universe = await Universe.fromFs(
			path.join(__dirname, './contracts'),
			{ canonicalOnly: true },
		);

		// The canonical version of the contract should exist
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				}),
			),
		).to.have.lengthOf(1);
		// But the alias should not
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'raspberrypi',
				}),
			),
		).to.have.lengthOf(0);
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'raspberrypi2',
				}),
			),
		).to.have.lengthOf(0);
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'intel-nuc',
				}),
			),
		).to.have.lengthOf(1);
	});

	it('allows filtering files when searching contracts', async () => {
		const universe = await Universe.fromFs(
			path.join(__dirname, './contracts'),
			{
				// Only load raspberrypi contracts
				filter: (filePath) =>
					path.basename(path.dirname(filePath)).startsWith('raspberry'),
			},
		);

		// The canonical version of the contract should exist
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'raspberry-pi',
				}),
			),
		).to.have.lengthOf(1);
		// But the alias should not
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'raspberrypi',
				}),
			),
		).to.have.lengthOf(1);
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'raspberrypi2',
				}),
			),
		).to.have.lengthOf(1);
		// Other contracts should not be loaded to the universe
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'intel-nuc',
				}),
			),
		).to.have.lengthOf(0);
		expect(
			universe.findChildren(
				Contract.createMatcher({
					type: 'hw.device-type',
					slug: 'jetson-nano',
				}),
			),
		).to.have.lengthOf(0);
	});
});
