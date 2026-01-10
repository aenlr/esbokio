# ES Kassa - Bokio

Ett program för att importera Z-Rapporter (dagsrapport/dagskassa) från
[dinkassa.se](https://www.dinkassa.se) till [Bokio](https://bokio.se).

## Funktioner

* Kontrollerar redan bokförda Z-Rapporter vilket också upptäcker när SIE-filer
  från ES Kassa redan importeras i Bokio.
* Laddar upp Z-Rapporten i PDF-format som underlag till verifikatet.
* Underlag (PDF, SIE4 och JSON) sparas som filer lokalt.
* Meny för att välja vad som ska importeras.

## Användning

Programmet kan köras utan argument och kommer att be om den information som krävs.

Ska man sedan köra det regelbundet kan man enklast använda miljövariabler för att inte
behöva ange all information varje gång.

```text
esbokio [OPTIONS]

Options:
  --dinkassa-username NAME     Användarnamn för dinkassa.se. (DINKASSA_USERNAME)
  --dinkassa-password NAME     Lösenord för dinkassa.se. (DINKASSA_PASSWORD)
  --bokio-api-token TOKEN      Token för privat integration i Bokio (BOKIO_API_TOKEN).
  --bokio-company-id UUID      Företagets ID i Bokio (BOKIO_COMPANY_ID).
                               OBS: Detta är inte företagets organisationsnummer utan det ID
                               som står i URL:en när man är inloggad i Bokio.

  --start YYYY-MM-DD           Startdatum för Z-Rapporter (standard är dagens datum)
  --end YYYY-MM-DD             Slutdatum för Z-Rapporter (standard är samma som startdatum)
  --date YYYY-MM-DD            Bearbeta Z-Rapporter för ett specifikt datum (standard är dagens datum)
  
Istället för absoluta datum tillåts även:
- today eller 0 = dagens datum
- yesterday = gårdagens datum
- week = första dagen i aktuell vecka
- month eller first = första dagen i aktuell månad
- -N = N dagar bakåt i tiden
```

## Guide

### ES Kassa / dinkassa.se

Använd din inloggning för https://www.dinkassa.se/v2

### Bokio

Skapa en privat integration under `Inställningar -> API Tokens`.
Ange t.ex. EasyCashier som integrationsnamn och klicks på *Lägg till integration*.

Kopiera token och spara på säker plats.

Kontrollera adressen i webbläsaren, den ser ut som något i stil med
`https://app.bokio.se/COMPANY-ID/settings-r/overview`.
Kopiera delen `COMPANY-ID` från adressen i webbläsaren som är delen
efter `https://app.bokio.se/` och nästföljande `/` (snedstreck).
Detta är ditt företags ID i Bokio (company id). Du behöver detta
när programmet körs.

### Miljövaribler

De användarnamn, lösenord, tokens och identiteter som du samlat ihop i 
från EasyCashier och Bokio kan du spara i en fil för att slippa ange dem
varje gång. Se till att bara behöriga har tillgång till filen.

Se [zrappimp.cmd](examples/zrappimp.cmd) för Windows och
[zrappimp.sh](examples/zrappimp.sh) för Linux,
macOs eller annat Un*x-likt OS.

Fyll i uppgifterna för ditt företag och konto.
