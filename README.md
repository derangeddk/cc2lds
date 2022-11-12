Country-code second-level domains (cc2LDs)
==========================================

I scoured the internet but could find no good single source for country-regulated second-level domains, such as `.gov.uk`, `.co.uk`, `.ac.uk`, etc.

I think it's probably quite useful to have this information in a single place, so I'm spending the time to put the current state together here in easily usable formats.

Please contribute if you find that anything is missing.

Goal:
- easy to contribute to (see [CONTRIBUTING.md](CONTRIBUTING.md))
- automatically build and distribute new version through various channels on change

## Accessing the datasets

I made this simple API that you can use to suffix search the most recent version of this dataset, running on Azure Functions and fine to use in production if your site doesn't have a huge amount of traffic (otherwise, warn me at [niels@deranged.dk](mailto:niels@deranged.dk)): <https://tld-api.deranged.dk/api/tlds> (try adding e.g. `?suffix=za` at the end of the URL to see suffix search).

Alternatively, the datasets are published - via npm - as JSON and YML to the following URLs:

- https://unpkg.com/cc2lds/output/2lds.json
- https://unpkg.com/cc2lds/output/2lds.yml

You can also install the latest dataset as an npm dependency:

```sh
npm i cc2lds
```

...which lets you import the data in your Javascript code:

```js
import cc2lds from 'cc2lds';
// `cc2lds` now contains the latest data
```


There are also tree-versions -- slightly denser and can be navigated more quickly once loaded into memory -- where you can check if a domain is generally managed (i.e. not open for general registration) by progressively checking its path:

```js
import cc2ldsTree from 'cc2lds/tree';

const uk = cc2ldsTree.uk; //truthy, so ".uk" is a regulated domain
const acUk = uk.ac; //truthy, so ".ac.uk" is a regulated domain
const huh = acUk.co; //falsy, so .co.ac.uk" is not a known regulated domain
```

You can use modern syntax for a nice little shorthand:

```js
if(cc2ldsTree.uk?.ac?) {
    //This block is reached, because ".ac.uk" is known and regulated.
}
```

These data formats can also be downloaded from unpkg:

- https://unpkg.com/cc2lds/output/2lds-tree.json
- https://unpkg.com/cc2lds/output/2lds-tree.yml

## FAQ

- **What about top-level domains?** IANA (the official authority) has [a nice central list of all TLDs](http://www.iana.org/domains/root/db), and it's even available as an [easily parseable text format](https://data.iana.org/TLD/tlds-alpha-by-domain.txt). The goal of this project is to make officially supported 2lds and 3lds as easily accessible as the above. Most countries don't have as easily parseable a list as IANA, though it would be neat if they did - then we could automate collecting everything!

## Read more

- [Second-level domain on Wikipedia](https://en.wikipedia.org/wiki/Second-level_domain)
