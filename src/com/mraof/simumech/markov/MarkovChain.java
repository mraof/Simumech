package com.mraof.simumech.markov;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;
import java.lang.instrument.Instrumentation;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Random;
import java.util.concurrent.locks.ReadWriteLock;
import java.util.concurrent.locks.ReentrantReadWriteLock;

import com.mraof.simumech.IntMap;
import com.mraof.simumech.Main;
import com.mraof.simumech.Profiler;
import com.mraof.simumech.Util;

public class MarkovChain 
{
	Random rand = new Random();
	//key is pair of two words
	HashMap<String, ArrayList<Integer>> wordPairsNext = new HashMap<String, ArrayList<Integer>>();
	//key is a single word, used if pair can't be found
	HashMap<String, ArrayList<Integer>> wordsNext = new HashMap<String, ArrayList<Integer>>();
	HashSet<String> lines = new HashSet<String>();

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
			ArrayList<String> splitLines = Util.split(". ");
			if(!lines.contains(line))
				lines.add(line);
			ArrayList<String> currentWords = Util.split(line);
			String previousWord = "";
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

				ArrayList<Integer> pairList = wordPairsNext.get(pair);
				if(pairList == null)
					pairList = new ArrayList<Integer>();
				pairList.add(wordIndex);
				wordPairsNext.put(pair, pairList);

				ArrayList<Integer> wordList = wordsNext.get(currentWord);
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
		for(int i = 0; i < currentWords.size() && sentence.size() < 2; i++)
		{
			String currentWord = currentWords.get(i);
			String pairKey = previousWord + " " + currentWord;
			ArrayList<Integer> list = wordPairsNext.get(pairKey);
			if(list != null && rand.nextDouble() > .1)
			{
				//				String word = words.get(list.get(rand.nextInt(list.size()))).toString();
				System.out.println("Adding " + pairKey + " to sentence from pair " + pairKey);
				if(sentence.size() == 0)
					sentence.add(pairKey);
				else sentence.add(currentWord);
			}
			else if(rand.nextDouble() > (1 / (currentWords.size() - i + 1) + .2))
			{
				String key = currentWord;
				list = wordsNext.get(key);
				if(list != null)
				{
					//					String word = words.get(list.get(rand.nextInt(list.size()))).toString();
					System.out.println("Adding " + key + " to sentence from word " + key);
					sentence.add(key);
				}
			}
			previousWord = currentWord;

		}
		if(sentence.isEmpty())
			sentence.add(currentWords.get(0));

		for(int size = sentence.size() - 1; size < sentence.size(); )
		{
			size = sentence.size();
			String currentWord = sentence.getLast();
			int wordIndex;
			if((wordIndex = currentWords.indexOf(previousWord)) != -1 && wordIndex < currentWords.size() - 1)
			{
				currentWord = currentWords.get(wordIndex + 1);
				currentWords.remove(wordIndex);
				currentWords.remove(wordIndex);
			}
			String key = previousWord + " " + currentWord;
			ArrayList<Integer> list = wordPairsNext.get(key);
			if(list != null && rand.nextDouble() < 4 / (double)sentence.size())
			{
				String word = words.get(list.get(rand.nextInt(list.size()))).toString();
				//								System.out.println("Adding " + word + " to sentence from pair " + key);
				sentence.add(word);
			}
			else /*if(rand.nextDouble() > (1 / (sentence.size() + .5) + .2))*/
			{
				key = currentWord;
				list = wordsNext.get(key);
				if(list != null)
				{
					String word = words.get(list.get(rand.nextInt(list.size()))).toString();
					//										System.out.println("Adding " + word + " to sentence from word " + key);
					sentence.add(word);
					int wordFrequency = 1;
					if(wordsNext.get(word) != null)
						wordFrequency = words.get(words.lookup(word)).count;
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

		replyString = sentence.pollFirst();
		if(!replyString.isEmpty())
			replyString = replyString.substring(0, 1).toUpperCase() + replyString.substring(1);
		if(replyString.equalsIgnoreCase(name))
			replyString = sender;
		for(String replyWord : sentence)
			if(!replyWord.isEmpty())
				replyString += " " + replyWord;
		return replyString;
	}

	public void load()
	{
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
		System.out.printf("Loaded %d lines, %d words, %d word pairs\n", lines.size(), wordsNext.size(), wordPairsNext.size());
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

