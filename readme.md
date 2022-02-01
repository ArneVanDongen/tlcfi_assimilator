# TLC-FI Assimilator

Bij deze een voorlopige versie van de TLC-FI Assimilator. Hiermee kun je SmartTraffic logging van TLC-FI berichten omzetten in een vlog bestand.

[![Coverage Status](https://coveralls.io/repos/github/ArneVanDongen/tlcfi_assimilator/badge.svg?branch=main)](https://coveralls.io/github/ArneVanDongen/tlcfi_assimilator?branch=main)

## Benodigdheden

* Een bestand met tlc-fi logging; valt te halen uit GCP Logging met de volgende query: `resource.labels.container_name="prod-${namespace}-communications-tlcfimessages"`, of als je het lokaal draait uit het bestand `tlcFiMesssages.log`.
* Een bestand met de mapping van Vlog IDs naar TLC-FI IDs. Je moet aan het einde van je commando de bestandsnaam meegeven, bijvoorbeeld: `tlcfi_assimilator iv3013-vlog-tlcfi-mapping.txt`. Zie hier een voorbeeld van een mapping file:

```
// TLC
iV3013

// Signals
0, 02
1, 04

// Detectors
0, D611
1, D612
```




## Optionele instellingen

* Een tijdstempel in ISO 8601 formaat van wanneer het VLog bestand wat gemaakt wordt moet beginnen. Zet deze met de commando optie `start-date-time`, bijvoorbeeld: `--start-date-time 2021-12-15T12:57:13.130`. Standaard wordt er gezocht naar het tijdstempel van het vroegst weggeschreven logbericht in het logbestand.
* Of het log bestand chronologisch is; dus van oud bovenaan naar nieuw onderaan. Standaard wordt er vanuit gegaan dat dit niet het geval is (dat is zo wanneer je logs uit GCP exporteert). Gebruik de commando optie `chronological` met een boolean waarde er achter. Bijvoorbeeld: `--chronological true`.
* De bestandsnaam van de TLC-FI logging. Standaard wordt er gezocht naar een bestand `tlcfi.txt` maar je kunt het instellen met de commando optie `tlcfi-log-file`, bijvoorbeeld: `--tlcfi-log-file tlcfi-snippet.txt`.


## Voorbeeld

Hier onder is een voorbeeld van het gebruik van de TLC-FI Assimilator. De gebruiker wil het bestand `tlcFiMessages.log` inladen wat gevuld is met logs van TestTerriFIQ. Hiervoor is een mapping bestand gemaakt genaamd `ttq-mapping.txt`. Al deze bestanden staan in dezelfde map als de TLC-FI Assimilator executable. De eerste log schrijving heeft een tijdstempel van 2021-12-15 12:57:13.130, dus wordt deze ook meegegeven.

```
tlcfi_assimilator --start-date-time 2021-12-15T12:57:13.130 --tlcfi-log-file tlcFiMessages.log ttq-mapping.txt
```
