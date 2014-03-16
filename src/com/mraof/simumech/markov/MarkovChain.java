package com.mraof.simumech.markov;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashSet;
import java.util.Random;
import java.util.concurrent.locks.ReadWriteLock;
import java.util.concurrent.locks.ReentrantReadWriteLock;
import java.util.regex.Pattern;

import com.mraof.simumech.IntMap;
import com.mraof.simumech.Main;
import com.mraof.simumech.Profiler;
import com.mraof.simumech.Util;

public class MarkovChain
{
	Random rand = new Random();
	//key is pair of two words
	HashMap<String, ArrayList<Integer>> wordPairsNext = new HashMap<String, ArrayList<Integer>>();
	HashMap<String, ArrayList<Integer>> wordPairsPrevious = new HashMap<String, ArrayList<Integer>>();
	//key is a single word, used if pair can't be found
	HashMap<String, ArrayList<Integer>> wordsNext = new HashMap<String, ArrayList<Integer>>();
	HashMap<String, ArrayList<Integer>> wordsPairsPrevious = new HashMap<String, ArrayList<Integer>>();
	LinkedHashSet<String> lines = new LinkedHashSet<String>();

	IntMap<String, Word> words = new IntMap<String, Word>();
	ReadWriteLock lock = new ReentrantReadWriteLock();

	public MarkovChain()
	{
		load();
		if(Profiler.instrumentation != null)
		{
			System.out.println("wordPairsNext: " + Profiler.deepSizeOf(wordPairsNext));
			System.out.println("wordsNext: " + Profiler.deepSizeOf(wordsNext));
			System.out.println("words: " + Profiler.deepSizeOf(words));
			System.out.println("lines: " + Profiler.deepSizeOf(lines));
		}
	}

	public void addLine(String line)
	{
		lock.writeLock().lock();
		{
			ArrayList<String> splitLines = Util.split(line, ". ", "\n");
			for(String currentLine : splitLines)
				if(!lines.contains(currentLine))
					lines.add(currentLine);
				else
					continue;
			ArrayList<String> currentWords = Util.split(line);
			String previousWord = "";
			ArrayList<Integer> pairList = null;
			ArrayList<Integer> wordList = null;
			for(int i = 0; i < currentWords.size() - 1; i++)
			{
				String currentWord = Util.selectivelyLowerCase(currentWords.get(i));
				String nextWord = Util.selectivelyLowerCase(currentWords.get(i + 1));
				String pair = previousWord + " " + currentWord;
				Integer wordIndex = words.lookup(nextWord);
				if(wordIndex == null)
					wordIndex = words.add(new Word(nextWord), nextWord);
				else
					words.get(wordIndex).increment();

				pairList = wordPairsNext.get(pair);
				if(pairList == null)
					pairList = new ArrayList<Integer>();
				pairList.add(wordIndex);
				wordPairsNext.put(pair, pairList);

				wordList = wordsNext.get(currentWord);
				if(wordList == null)
					wordList = new ArrayList<Integer>();

				wordList.add(wordIndex);
				wordsNext.put(currentWord, wordList);

				previousWord = currentWord;
				//				System.out.println("\"" + currentWords.get(i) + "\",");
			}
		}
		lock.writeLock().unlock();
	}

	public String reply(String message)
	{
		return reply(message, "", "");
	}
	public String reply(String inputString, String name, String sender)
	{
		ArrayList<String> currentLines;
		ArrayList<String> currentWords = new ArrayList<String>();
		ArrayDeque<String> sentence = new ArrayDeque<String>();
		String replyString = "";
		int inputLength = inputString.length();

		if(inputString.isEmpty())
		{
			System.out.println("Empty input string");
			return "";
		}

		currentLines = Util.split(inputString, ". ");

		for(int i = 0; i < currentLines.size(); i++)
			currentWords.addAll(Util.split(currentLines.get(i)));
		for(int i = 0; i < currentWords.size(); i++)
			currentWords.set(i, Util.selectivelyLowerCase(currentWords.get(i)));
		if(currentWords.isEmpty())
		{
			System.out.println("Input string contained no words");
			return "";
		}

		lock.readLock().lock();
		String previousWord = "";
		int bestWord = 0;
		int bestWordPair = 0;
		String bestWordPairString = " " + currentWords.get(0);
		for(int i = 0; i < currentWords.size(); i++)
		{
			String currentWord = currentWords.get(i);
			String pairKey = previousWord + " " + currentWord;
			ArrayList<Integer> list = wordPairsNext.get(pairKey);
			ArrayList<Integer> bestList = wordPairsNext.get(bestWordPairString);
			if(bestList == null || bestList.size() == 0 || (list != null && list.size() > 0 && (list.size() < bestList.size())))
			{
				bestWordPairString = pairKey;
				bestWordPair = i;
				//System.out.println("Best word pair " + bestList + " " + list + " " + bestWordPairString + " " + bestWordPair);
			}
			list = wordsNext.get(currentWord);
			bestList = wordsNext.get(currentWords.get(bestWord));
			if(bestList == null || bestList.size() == 0 || (list != null && list.size() > 0 && list.size() < bestList.size()))
			{
				bestWord = i;
				//System.out.println("Best word " + bestList + " " + list + " " + bestWord);
			}

			previousWord = currentWord;

		}
		ArrayList<Integer> bestList;
		if((bestList = wordPairsNext.get(bestWordPairString)) != null && bestList.size() > 0 && rand.nextDouble() > .05)
		{
			//				String word = words.get(list.get(rand.nextInt(list.size()))).toString();
			//System.out.println("Adding \"" + bestWordPairString + "\" to start the sentence");
			if(bestWordPairString.charAt(0) != ' ' && bestWordPair > 0)
			{
				sentence.add(currentWords.get(bestWordPair - 1));
				previousWord = currentWords.get(bestWordPair - 1);
				sentence.add(currentWords.get(bestWordPair));
			}
			else
			{
				sentence.add(currentWords.get(bestWordPair));
				previousWord = " ";
			}
		}
		else
		{
			String key = currentWords.get(bestWord);
			bestList = wordsNext.get(key);
			if(bestList != null)
			{
				//					String word = words.get(list.get(rand.nextInt(list.size()))).toString();
			//	System.out.println("Adding " + key + " to sentence from word " + key);
				sentence.add(key);
			}
		}

		if(sentence.isEmpty())
			sentence.add(currentWords.get(0));

		for(int size = sentence.size() - 1; size < sentence.size(); )
		{
			size = sentence.size();
			String currentWord = sentence.getLast();
			int wordIndex;
			Word wordObj = words.get(words.lookup(currentWord));
//			if(wordObj != null)
//				System.out.println(wordObj.getCount() / (double) lines.size());
			if((wordIndex = currentWords.indexOf(previousWord)) != -1 && wordIndex < currentWords.size() - 1 && wordObj != null && wordObj.getCount() / (double) lines.size() < rand.nextDouble())
			{
				currentWord = currentWords.get(wordIndex + 1);
				currentWords.remove(wordIndex);
				currentWords.remove(wordIndex);
			}
			String key = previousWord + " " + currentWord;
			ArrayList<Integer> list = wordPairsNext.get(key);
//			System.out.println(key + ", " + list);
			if(list != null && rand.nextDouble() < inputLength / (double)sentence.size())
			{
				String word = words.get(list.get(rand.nextInt(list.size()))).toString();
				//System.out.println("Adding " + word + " to sentence from pair " + key);
				sentence.add(word);
			}
			else if(rand.nextDouble() < (1 / (sentence.size() + .5)))
			{
				key = currentWord;
				list = wordsNext.get(key);
				if(list != null)
				{
					String word = words.get(list.get(rand.nextInt(list.size()))).toString();
					//System.out.println("Adding " + word + " to sentence from word " + key);
					sentence.add(word);
					int wordFrequency = 1;
					if(wordsNext.get(word) != null)
						wordFrequency = words.get(words.lookup(word)).getCount();
//										System.out.println(word + ": " + wordFrequency / (double)sentence.size());
					if(rand.nextDouble() > (wordFrequency / (double)sentence.size()))
					{
						break;
					}
				}
			}
			previousWord = currentWord;

		}
		lock.readLock().unlock();

		do
			replyString = sentence.pollFirst();
		while(replyString.isEmpty());

		if(!replyString.isEmpty())
			replyString = replyString.substring(0, 1).toUpperCase() + replyString.substring(1);
		if(replyString.equalsIgnoreCase(name) && !sender.isEmpty())
			replyString = sender;
		for(String replyWord : sentence)
			if(!replyWord.isEmpty())
				replyString += " " + replyWord;
		return replyString;
	}
	public String command(String command)
	{
		ArrayList<String> parts = Util.split(command);
		switch(parts.get(0).toUpperCase())
		{
		case "STATS":
			return "Word pair contexts: " + wordPairsNext.size() + ", word contexts: " + wordsNext.size() + ", words: " + words.size() + ", lines: " + lines.size();
		case "LINES":
			if(parts.size() > 1)
			{
				String word = parts.get(1).toLowerCase();
				String matchingLines = "";
				int matches = 0;
				for(String line : lines)
					if(line.toLowerCase().matches(".*\\b(" + Pattern.quote(word) + ")\\b.*"))
					{
						matchingLines += "\n" + line;
						matches++;
					}
				if(matches < 10 || parts.size() > 2 && parts.get(2).equalsIgnoreCase("YES"))
					return "Found \"" + word + "\" " + matches + " times" + matchingLines;
			}
		case "KNOWN":
			if(parts.size() > 1)
			{
				String word = parts.get(1).toLowerCase();
				int matches = 0;
				for(String line : lines)
					if(line.toLowerCase().matches(".*\\b(" + Pattern.quote(word) + ")\\b.*"))
						matches++;
					return "Found \"" + word + "\" " + matches + " times";
			} else return "Invalid syntax, should be " + parts.get(0).toUpperCase() + " <word>";
		case "WORDSTATS":
			if(parts.size() > 1)
			{
				String wordString = Util.selectivelyLowerCase(parts.get(1));
				if(words.lookup(wordString) != null)
				{
					Word word = words.get(words.lookup(wordString));
					String nextWordString = "";
					int nextWordCount = 0;
					ArrayList<Integer[]> nextWords = new ArrayList<Integer[]>();
					if(wordsNext.get(wordString.toLowerCase()) != null)
						for(Integer index : wordsNext.get(wordString.toLowerCase()))
						{
							boolean notFound = true;
							for(int i = 0; i < nextWords.size(); i++)
								if(nextWords.get(i)[0] == index)
								{
									nextWords.get(i)[1]++;
									if(nextWords.get(i)[1] > nextWordCount)
									{
										nextWordCount = nextWords.get(i)[1];
										nextWordString = words.get(nextWords.get(i)[0]).toString();
									}
									notFound = false;
									break;
								}
							if(notFound)
								nextWords.add(new Integer[]{index, 1});
						}

					return "\"" + wordString + "\" has a count of " + word.getCount() + ", an index of " + words.lookup(wordString) + ", with a most common next word of \"" + nextWordString + "\" (" + nextWordCount + " times)";
				}
				else return wordString + " is not known";
			}
		case "WORDID":
			if(parts.size() > 1)
			{
				try
				{
					Integer index = Integer.decode(parts.get(1));
					return words.get(index).toString();
				} catch(ArrayIndexOutOfBoundsException e) {return "ID out of bounds";} catch(NumberFormatException e) {return "\"" + parts.get(1) + "\" is not a number";}
			}
		}
		
		return "";
	}
	public void load()
	{
		long startTime = System.currentTimeMillis();
		try {
			BufferedReader reader = new BufferedReader(new FileReader("lines.txt"));
			String line;
			while((line = reader.readLine()) != null)
			{
				addLine(line);
				if(Main.useCR)
					System.out.printf("Added %d lines\r", lines.size());
			}
			System.out.println();
			reader.close();
		} catch (FileNotFoundException e) {e.printStackTrace();} catch (IOException e) {e.printStackTrace();}
		long endTime = System.currentTimeMillis();

		System.out.printf("Loaded %d lines, %d words, %d word pairs in %d milliseconds\n", lines.size(), wordsNext.size(), wordPairsNext.size(), endTime - startTime);
	}
	public void save()
	{
		//		try {
		//			Files.copy(new File("lines.txt").toPath(), new File("lines.bak.txt").toPath());
		//		} catch (IOException e) {System.err.println("Unable to backup file before saving");}
		lock.readLock().lock();
		try {
			BufferedWriter writer = new BufferedWriter(new FileWriter("lines.txt"));
			for(String line : lines)
			{
				writer.write(line);
				writer.newLine();
			}
			writer.close();
		} catch (IOException e) {
			e.printStackTrace();
		}
		lock.readLock().unlock();
	}

}

