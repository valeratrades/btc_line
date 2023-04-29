import requests, tempfile, json, os

tempdir = tempfile.gettempdir()

response = requests.get("https://www.cftc.gov/dea/futures/financial_lf.htm")

lines = response.text.splitlines()
for num_s, s in  enumerate(lines):
    if (s.find('#133741') != -1):
        line_positions = lines[num_s+2]
        line_changeFrom = lines[num_s+5]
        break

def find45(line):
    numbers = []
    help_string = ''
    for l_num, l in enumerate(line):
        if(l != ' '):
            help_string += l
            if(line[l_num+1] == ' '):
                numbers.append(help_string)
                help_string = ''
        if len(numbers) >= 5:
            break
    return numbers[3], numbers[4]

out_str = ''
out_str += str(find45(line_positions)) + str(find45(line_changeFrom))

large_window_config = json.load(open(os.path.join(tempdir, 'large_window.json'), 'r'))
large_window_config['CMEpositions'] = out_str
json.dump(large_window_config, open(os.path.join(tempdir, 'large_window.json'), 'w')) 
