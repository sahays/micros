# Story: PDF Generation

- [ ] **Status: Planning**
- **Epic:** [001-invoicing-service](../epics/001-invoicing-service.md)

## Summary

Implement GenerateInvoicePdf, GenerateReceiptPdf, and GenerateStatementPdf gRPC methods for document generation.

## Tasks

- [ ] Define proto messages: GenerateInvoicePdfRequest/Response
- [ ] Define proto messages: GenerateReceiptPdfRequest/Response
- [ ] Define proto messages: GenerateStatementPdfRequest/Response
- [ ] Select PDF library (printpdf, typst, or wkhtmltopdf)
- [ ] Design invoice PDF template
- [ ] Design receipt PDF template
- [ ] Design statement PDF template
- [ ] Implement PDF generation service
- [ ] Implement document storage (local or document-service)

## gRPC Methods

### GenerateInvoicePdf
**Input:** tenant_id, invoice_id
**Output:** pdf_url or pdf_bytes

**Content:**
- Invoice header (number, dates, status)
- Customer details (name, address)
- Line items table (description, qty, price, tax, amount)
- Totals (subtotal, tax, total)
- Payment terms and notes

### GenerateReceiptPdf
**Input:** tenant_id, receipt_id
**Output:** pdf_url or pdf_bytes

**Content:**
- Receipt header (number, date)
- Customer name
- Invoice reference
- Amount paid
- Payment method and reference

### GenerateStatementPdf
**Input:** tenant_id, statement_id
**Output:** pdf_url or pdf_bytes

**Content:**
- Statement header (period, customer)
- Opening balance
- Transaction lines (date, reference, amount, balance)
- Closing balance
- Aging summary (optional)

## PDF Storage Options

**Option A: Return bytes**
- Return PDF as bytes in response
- Client handles storage/display
- Simpler implementation

**Option B: Store and return URL**
- Store PDF via document-service
- Return signed URL for download
- Better for large documents

## Acceptance Criteria

- [ ] GenerateInvoicePdf returns valid PDF
- [ ] Invoice PDF contains all required fields
- [ ] GenerateReceiptPdf returns valid PDF
- [ ] Receipt PDF contains payment details
- [ ] GenerateStatementPdf returns valid PDF
- [ ] Statement PDF contains all lines
- [ ] PDFs render correctly in viewers
- [ ] Currency and numbers formatted correctly

## Integration Tests

- [ ] Generate invoice PDF for issued invoice
- [ ] Generate receipt PDF for payment
- [ ] Generate statement PDF for period
- [ ] PDF contains correct data
- [ ] PDF is valid (parseable)
